use std::borrow::Cow;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use itertools::Itertools;
use libafl::executors::command::StdCommandConfigurator;
use libafl::prelude::*;
use libafl_bolts::prelude::*;
use mutatis::Session;
use sb3::StructuredProject;

#[derive(Clone, Debug, PartialEq, Eq)]
struct CommandOutcome {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[derive(Debug)]
struct Config {
    command: Vec<String>,
    input_path: PathBuf,
    output_path: Option<PathBuf>,
    max_rounds: usize,
}

fn usage(bin_name: &str) {
    eprintln!(
        "Usage: {bin_name} -- <command> [args...] --input <project.json> [--output \
         <reduced.json>] [--max-rounds N]"
    );
}

fn parse_args() -> Result<Config, String> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--") => {}
        _ => return Err("expected `--` before the command".into()),
    }

    let mut command = Vec::new();
    let mut input_path = None;
    let mut output_path = None;
    let mut max_rounds = 1024usize;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => {
                let Some(path) = args.next() else {
                    return Err("missing value for --input".into());
                };
                input_path = Some(PathBuf::from(path));
            }
            "--output" => {
                let Some(path) = args.next() else {
                    return Err("missing value for --output".into());
                };
                output_path = Some(PathBuf::from(path))
            }
            "--max-rounds" => {
                let Some(value) = args.next() else {
                    return Err("missing value for --max-rounds".into());
                };
                max_rounds = value
                    .parse::<usize>()
                    .map_err(|_| "--max-rounds must be a number".to_string())?;
            }
            other => command.push(other.to_string()),
        }
    }

    if command.is_empty() {
        return Err("missing command to execute".into());
    }

    let Some(input_path) = input_path else {
        return Err("missing --input <project.json>".into());
    };

    Ok(Config {
        command,
        input_path,
        output_path,
        max_rounds,
    })
}

fn read_project(
    path: &Path,
) -> Result<sb3::structured::StructuredProject, Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(path)?;
    let raw = sb3::raw::Sb3Project::try_from(json.as_str())?;
    Ok(sb3::structured::StructuredProject::try_from(raw)?)
}

fn project_json(project: &sb3::structured::StructuredProject) -> Option<String> {
    let raw = sb3::raw::Sb3Project::try_from(project.clone()).ok()?;
    serde_json::to_string(&raw).ok()
}

fn write_temp_project_json(
    project: &sb3::structured::StructuredProject,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let Some(json) = project_json(project) else {
        return Err("failed to serialize baseline project".into());
    };

    let mut path = std::env::temp_dir();
    path.push(format!(
        "hyperquark-reduce-project-{}-{}.json",
        std::process::id(),
        current_nanos()
    ));

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(json.as_bytes())?;

    Ok(path)
}

fn capture_baseline(
    command: &[String],
    project: &sb3::structured::StructuredProject,
) -> Result<CommandOutcome, Box<dyn std::error::Error>> {
    let temp_path = write_temp_project_json(project)?;
    // let Some(json) = project_json(project) else {
    //     return Err("failed to serialize baseline project".into());
    // };

    let mut child = Command::new(&command[0]);
    child.args(&command[1..]);
    child.arg(dbg!(&temp_path));
    // child.arg(&json);

    println!("{command:?}");

    let output = child.output()?;
    let outcome = CommandOutcome {
        status: output.status.code(),
        stdout: output.stdout,
        stderr: output.stderr,
    };

    Ok(outcome)
}

#[derive(Clone)]
struct CommandObserver<'a> {
    stdout: Handle<StdOutObserver>,
    stderr: Handle<StdErrObserver>,
    baseline: &'a CommandOutcome,
    // best_candidate: Arc<Mutex<Option<(usize, StructuredProject)>>>,
}

impl<'a> CommandObserver<'a> {
    fn new(
        stdout: Handle<StdOutObserver>,
        stderr: Handle<StdErrObserver>,
        baseline: &'a CommandOutcome,
    ) -> Self {
        Self {
            stdout,
            stderr,
            baseline,
            // best_candidate: Arc::new(Mutex::new(None)),
        }
    }

    // fn best_candidate(&self) -> Option<(usize, StructuredProject)> {
    //     self.best_candidate.lock().ok()?.clone()
    // }

    fn stdout(&self) -> &Handle<StdOutObserver> {
        &self.stdout
    }

    fn stderr(&self) -> &Handle<StdErrObserver> {
        &self.stderr
    }
}

impl<'a> Named for CommandObserver<'a> {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("CommandObserver")
    }
}

impl<'a, S> StateInitializer<S> for CommandObserver<'a> {}

impl<'a, EM, OT, S> Feedback<EM, StructuredProject, OT, S> for CommandObserver<'a>
where
    OT: MatchNameRef,
{
    fn is_interesting(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        input: &StructuredProject,
        observers: &OT,
        _exit_kind: &ExitKind,
    ) -> Result<bool, Error> {
        let stdout = observers.get(&self.stdout).unwrap().output.clone();
        let stderr = observers.get(&self.stderr).unwrap().output.clone();
        // println!("{:?}", stdout.as_ref().cloned().map(String::from_utf8));
        // println!("{:?}", stderr.as_ref().cloned().map(String::from_utf8));
        // println!("{:?}", String::from_utf8(self.baseline.stdout.clone()));
        // println!("{:?}", String::from_utf8(self.baseline.stderr.clone()));
        // println!("{}", input.len());
        let interesting = stdout.clone().unwrap_or_default() == self.baseline.stdout
            && stderr.clone().unwrap_or_default() == self.baseline.stderr;
        let input_len = input.len();

        // println!("{input_len}");
        // println!("{interesting}");
        Ok(interesting)

        // Ok(interesting)
    }
}

impl<'a, T> FeedbackFactory<CommandObserver<'a>, T> for CommandObserver<'a> {
    fn create_feedback(&self, _ctx: &T) -> CommandObserver<'a> {
        self.clone()
    }
}

type ScratchCommandState = StdState<
    InMemoryCorpus<StructuredProject>,
    StructuredProject,
    RomuDuoJrRand,
    InMemoryCorpus<StructuredProject>,
>;
type ScratchCommandObservers = tuple_list!(StdOutObserver, StdErrObserver);
type ScratchCommandFuzzer<'a> =
    StdFuzzer<QueueScheduler, (), NopBytesConverter, NopInputFilter, ()>;

struct ScratchCommandExecutor {
    cmd_executor: CommandExecutor<
        StructuredProject,
        ScratchCommandObservers,
        ScratchCommandState,
        StdCommandConfigurator,
    >,
}

impl ScratchCommandExecutor {
    fn new(
        command: &[String],
        cmd_observer: &CommandObserver,
        observers: ScratchCommandObservers,
    ) -> Self {
        let ex = CommandExecutor::builder()
            .program(command.first().unwrap())
            .args(command.iter().dropping(1).collect::<Box<[_]>>())
            .arg_input_file(format!(
                "{}/hyperquark-reduce-project-{}-{}.json",
                std::env::temp_dir().display(),
                std::process::id(),
                current_nanos()
            ))
            .stdout_observer(cmd_observer.stdout().clone())
            .stderr_observer(cmd_observer.stderr().clone())
            .build(observers)
            .unwrap();
        Self { cmd_executor: ex }
    }
}

impl HasObservers for ScratchCommandExecutor {
    type Observers = ScratchCommandObservers;

    fn observers(&self) -> RefIndexable<&Self::Observers, Self::Observers> {
        self.cmd_executor.observers()
    }

    fn observers_mut(&mut self) -> RefIndexable<&mut Self::Observers, Self::Observers> {
        self.cmd_executor.observers_mut()
    }
}

impl<EM> Executor<EM, StructuredProject, ScratchCommandState, ScratchCommandFuzzer<'_>>
    for ScratchCommandExecutor
{
    fn run_target(
        &mut self,
        fuzzer: &mut ScratchCommandFuzzer,
        state: &mut ScratchCommandState,
        mgr: &mut EM,
        input: &StructuredProject,
    ) -> Result<ExitKind, Error> {
        self.cmd_executor.run_target(fuzzer, state, mgr, input)
    }
}

struct StructuredProjectMutator;

impl Named for StructuredProjectMutator {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("StructuredProjectMutator")
    }
}

impl<S> Mutator<StructuredProject, S> for StructuredProjectMutator
where
    S: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut StructuredProject,
    ) -> Result<MutationResult, Error> {
        let seed = state.rand_mut().next();

        // println!("{}", input.len());

        let mut session = Session::new().shrink(true).seed(seed);

        session.mutate(input).unwrap();

        // println!("mutated: {}", input.len());

        Ok(MutationResult::Mutated)
    }

    fn post_exec(&mut self, _state: &mut S, _new_corpus_id: Option<CorpusId>) -> Result<(), Error> {
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    let bin_name = args.next().unwrap_or_else(|| "reduce_project".into());
    let config = match parse_args() {
        Ok(config) => config,
        Err(message) => {
            usage(&bin_name);
            return Err(message.into());
        }
    };

    let base_case = read_project(&config.input_path)?;
    let baseline = capture_baseline(&config.command, &base_case)?;

    println!(
        "baseline captured: status={:?}, stdout={} bytes, stderr={} bytes",
        baseline.status,
        baseline.stdout.len(),
        baseline.stderr.len()
    );
    println!("{}", String::from_utf8(baseline.stdout.clone()).unwrap());
    println!("{}", String::from_utf8(baseline.stderr.clone()).unwrap());

    let stdout = StdOutObserver::new("stdout".into())?;
    let stderr = StdErrObserver::new("stderr".into())?;

    let mut cmd_observer = CommandObserver::new(stdout.handle(), stderr.handle(), &baseline);

    let observers = tuple_list!(stdout, stderr);

    let mut executor = ScratchCommandExecutor::new(&config.command, &cmd_observer, observers);

    let monitor = SimpleMonitor::new(|msg| println!("{msg}"));
    let mut event_manager = SimpleEventManager::new(monitor);

    // let mutator = StructuredProjectMutator;

    // let stage: StdTMinMutationalStage<ScratchCommandExecutor, _, _, _, _, _, _, _> =
    // StdTMinMutationalStage::new(mutator, cmd_observer.clone(), config.max_rounds);

    let mut corpus = InMemoryCorpus::<StructuredProject>::new();
    corpus.add(Testcase::new(base_case.clone()))?;

    let mut state = StdState::new(
        StdRand::with_seed(current_nanos()),
        corpus,
        InMemoryCorpus::<StructuredProject>::new(),
        &mut (),
        &mut (),
    )?;

    let scheduler = QueueScheduler::new();
    let mut fuzzer: ScratchCommandFuzzer = StdFuzzer::new(scheduler, (), ());

    // state.generate_initial_inputs_forced(
    //     &mut fuzzer,
    //     &mut executor,
    //     SingletonGenerator(base_case),
    //     &mut event_manager,
    //     1,
    // )?;

    // stage.perform(&mut fuzzer, &mut executor, &mut state, &mut event_manager)?;

    // fuzzer.fuzz_one(
    //     &mut tuple_list!(stage),
    //     &mut executor,
    //     &mut state,
    //     &mut event_manager,
    // )?;

    let mut i = 0;
    let mut input = base_case;
    let mut best_len = input.len();
    let mut session = Session::new().shrink(true);
    let output_path: PathBuf = config.output_path.unwrap_or(config.input_path);

    loop {
        if i == config.max_rounds {
            break;
        }

        i += 1;

        let mut this_input = input.clone();

        session.mutate(&mut this_input).unwrap();

        if this_input.len() >= best_len {
            continue;
        }

        let exit_kind =
            fuzzer.execute_input(&mut state, &mut executor, &mut event_manager, &this_input)?;

        let (_, corpus_id) = fuzzer.evaluate_execution(
            &mut state,
            &mut event_manager,
            &this_input,
            &*executor.observers(),
            &exit_kind,
            false,
        )?;

        let interesting = cmd_observer.is_interesting(
            &mut state,
            &mut event_manager,
            &this_input,
            &*executor.observers(),
            &exit_kind,
        )?;

        if interesting {
            i = 0;
            input = this_input;
            best_len = input.len();
            println!("new best: {best_len}");
            std::fs::write(output_path.clone(), project_json(&input).unwrap())?;
        }
    }

    // let Some((best_len, best_project)) = cmd_observer.best_candidate() else {
    //     return Err("tmin did not record a minimized candidate".into());
    // };

    let best_json = project_json(&input).ok_or("failed to serialize minimized project")?;
    println!("{best_len:?}");

    Ok(())
}
