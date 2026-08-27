use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;
use std::collections::BTreeSet;

use arbitrary::Arbitrary;
use hyperquark::error::HQErrorType;
use hyperquark::wasm::flags::all_wasm_features;
use libafl::Fuzzer;
use libafl::corpus::{Corpus, InMemoryCorpus, OnDiskCorpus, Testcase};
use libafl::events::SimpleEventManager;
use libafl::executors::ExitKind;
use libafl::executors::inprocess::InProcessExecutor;
use libafl::feedbacks::{CrashFeedback, TimeFeedback};
use libafl::fuzzer::StdFuzzer;
use libafl::generators::RandBytesGenerator;
use libafl::inputs::{BytesInput, HasTargetBytes};
use libafl::monitors::SimpleMonitor;
use libafl::mutators::havoc_mutations;
use libafl::mutators::scheduled::HavocScheduledMutator;
use libafl::observers::TimeObserver;
use libafl::schedulers::QueueScheduler;
use libafl::stages::mutational::StdMutationalStage;
use libafl::state::{HasCorpus, StdState};
use libafl_bolts::current_nanos;
use libafl_bolts::rands::StdRand;
use libafl_bolts::tuples::tuple_list;

const INITIAL_INPUTS: usize = 64;
const MAX_INPUT_SIZE: usize = 16 * 1024;
const ARTIFACT_DIR: &str = "crash-artifacts";

fn recover_artifacts(data: &[u8]) -> Option<(String, String)> {
    let mut unstructured = arbitrary::Unstructured::new(data);

    let structured = sb3::structured::StructuredProject::arbitrary(&mut unstructured).ok()?;
    let raw_project: sb3::raw::Sb3Project = structured.clone().try_into().ok()?;

    let json = serde_json::to_string_pretty(&raw_project).ok()?;
    let structured_txt = format!("{structured:#?}");

    Some((json, structured_txt))
}

fn process_crash_file(crash_path: &PathBuf, artifact_dir: &PathBuf) {
    let Some(stem) = crash_path.file_name().and_then(|name| name.to_str()) else {
        return;
    };

    if stem.starts_with('.') || stem.ends_with(".metadata") {
        return;
    }

    let json_path = artifact_dir.join(format!("{stem}.json"));
    let txt_path = artifact_dir.join(format!("{stem}.txt"));

    if json_path.exists() && txt_path.exists() {
        return;
    }

    let Ok(data) = std::fs::read(crash_path) else {
        return;
    };

    let Some((json, structured_txt)) = recover_artifacts(&data) else {
        return;
    };

    let _ = std::fs::write(json_path, json);
    let _ = std::fs::write(txt_path, structured_txt);
}

fn process_crash_directory(crash_dir: &PathBuf, artifact_dir: &PathBuf) {
    let Ok(entries) = std::fs::read_dir(crash_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            process_crash_file(&path, artifact_dir);
        }
    }
}

fn crash_basenames(crash_dir: &PathBuf) -> BTreeSet<String> {
    let mut stems = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(crash_dir) else {
        return stems;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(stem) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if stem.starts_with('.') || stem.ends_with(".metadata") {
            continue;
        }
        stems.insert(stem.to_string());
    }

    stems
}

fn compile_input(data: &[u8], flags: hyperquark::wasm::WasmFlags) {
    let mut unstructured = arbitrary::Unstructured::new(data);
    let Ok(structured) = sb3::structured::StructuredProject::arbitrary(&mut unstructured) else {
        return;
    };

    let Ok(raw_project): Result<sb3::raw::Sb3Project, _> = structured.try_into() else {
        return;
    };

    let Ok(json) = serde_json::to_string(&raw_project) else {
        return;
    };

    match hyperquark::sb3_to_wasm(&json, flags) {
        Ok(_) => (),
        Err(err) => match err.err_type {
            HQErrorType::InternalError => {
                panic!("{} at {}:{}:{}", err.msg, err.file, err.line, err.column)
            }
            _ => (),
        },
    }
}

fn main() -> Result<(), libafl::Error> {
    std::fs::create_dir_all("corpus")?;
    std::fs::create_dir_all("crashes")?;
    std::fs::create_dir_all(ARTIFACT_DIR)?;

    let crash_dir = PathBuf::from("crashes");
    let artifact_dir = PathBuf::from(ARTIFACT_DIR);

    // Backfill artifacts for existing crash files from previous runs.
    process_crash_directory(&crash_dir, &artifact_dir);

    // Keep generating artifacts as new crashes are written.
    {
        let watch_crash_dir = crash_dir.clone();
        let watch_artifact_dir = artifact_dir.clone();
        std::thread::spawn(move || {
            let mut seen = crash_basenames(&watch_crash_dir);
            loop {
                process_crash_directory(&watch_crash_dir, &watch_artifact_dir);

                let current = crash_basenames(&watch_crash_dir);
                for stem in current.difference(&seen) {
                    println!("New crash basename: {stem}");
                }
                seen = current;

                std::thread::sleep(Duration::from_millis(500));
            }
        });
    }

    let monitor = SimpleMonitor::new(|msg| println!("{msg}"));
    let mut event_manager = SimpleEventManager::new(monitor);

    let time_observer = TimeObserver::new("exec_time");
    let mut feedback = TimeFeedback::new(&time_observer);
    let mut objective = CrashFeedback::new();

    let mut state = StdState::new(
        StdRand::with_seed(current_nanos()),
        InMemoryCorpus::<BytesInput>::new(),
        OnDiskCorpus::new("crashes")?,
        &mut feedback,
        &mut objective,
    )?;

    let scheduler = QueueScheduler::new();
    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

    let wasm_flags = hyperquark::wasm::WasmFlags::new(all_wasm_features());

    let mut harness = move |input: &BytesInput| {
        let bytes = input.target_bytes();
        compile_input(bytes.as_ref(), wasm_flags);
        ExitKind::Ok
    };

    let mut executor = InProcessExecutor::new(
        &mut harness,
        tuple_list!(time_observer),
        &mut fuzzer,
        &mut state,
        &mut event_manager,
    )?;

    // Try to load user-provided corpus files first.
    let seed_dirs = [PathBuf::from("corpus")];
    let _ = state.load_initial_inputs(&mut fuzzer, &mut executor, &mut event_manager, &seed_dirs);

    if state.must_load_initial_inputs() {
        let mut generator = RandBytesGenerator::new(NonZeroUsize::new(MAX_INPUT_SIZE).unwrap());
        state.generate_initial_inputs(
            &mut fuzzer,
            &mut executor,
            &mut generator,
            &mut event_manager,
            INITIAL_INPUTS,
        )?;
    }

    // Some feedback setups may reject all generated inputs; ensure corpus is never empty.
    if state.corpus().count() == 0 {
        let fallback_inputs = [
            Vec::new(),
            vec![0_u8],
            b"{}".to_vec(),
            b"{\"targets\":[]}".to_vec(),
        ];
        for data in fallback_inputs {
            state
                .corpus_mut()
                .add(Testcase::new(BytesInput::new(data)))?;
        }
    }

    let mutator = HavocScheduledMutator::new(havoc_mutations());
    let mut stages = tuple_list!(StdMutationalStage::new(mutator));

    fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut event_manager)
}
