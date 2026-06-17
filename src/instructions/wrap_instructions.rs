#![allow(
    dead_code,
    reason = "staged implementation; this wrapper is not wired in yet"
)]
// This module uses typm to document invariants and other correctness conditions;
// view the compiled rustdoc pages in a browser for an easier read.
// https://hyperquark.edgecompute.app/docs/internal/hyperquark/instructions/wrap_instructions/index.html,
// or use `./build.sh -D && npm run dev` to view locally.

//! Module for generating WASM for a sequence of instructions.
//!
//! Some definitions for formality:
//!
//! Define ``{$#`is_root` : #`Fork` -> #`bool`$}`` to be
//!   ``{$ #`is_root`;(f) = f.#`branch`.#`is_none`;(). $}``
//!
//! Let `{$F$}` be the function mapping a [`Fork`] to all [`Fork`]s reachable from it,
//! with `{$F(f)$}` a shorthand for ``{$F(#`Some`;(f))$}``:
//! ```typm-render
//!   F(#`None`)    &= {}\
//!   F(#`Some`;(f)) &= { f } union
//!     F(#`Some`;(f.#`branch`?.#`b_if`)) union
//!     F(f.#`branch`?.#`b_else`).
//! ```
//!
//! Let `{$L$}` be the function mapping a [`Fork`] to its leaves:
//! ```typm-render
//! L(f) = { g in F(f) | #`is_root`;(g) }.
//! ```
//!
//! These definitions are used in pre-/post-conditions and invariants for
//! functions and datatypes.

use core::cell::RefMut;

use wasm_encoder::{BlockType, ConstExpr, Instruction as WInstruction, ValType};
use wasm_gen::wasm;

use crate::instructions::IrOpcode;
use crate::instructions::prelude::*;
use crate::ir::{IrType, ReturnType, TypeStack};
use crate::wasm::{GlobalExportable, GlobalMutable, StepFunc, StringsTable, WasmProject};

/// An if/else branch. Invariant `{$I_b$}` declared here.
///
/// # Invariant `{$I_b$}`
/// ```typm-render
/// [exists #`b_else` . #`branch`.#`b_else` = #`Some`;(#`b_else`)]\
/// ==> #`b_else`.#`entry` = #`branch`.#`b_if`.#`entry`.
/// ```
#[derive(Debug, Clone)]
struct Branch {
    condition: Vec<InternalInstruction>,
    b_if: Rc<Fork>,
    /// `None` indicates an unreachable branch
    b_else: Option<Rc<Fork>>,
}

impl Branch {
    fn collapse_with_returns(
        &self,
        func: &StepFunc,
        wasm: &mut Vec<InternalInstruction>,
        returns: &Vec<ValType>,
    ) -> HQResult<()> {
        wasm.extend(self.condition.iter().cloned());

        let block_type = func.registries().types().function(
            self.b_if
                .entry
                .clone()
                .map(WasmProject::ir_type_to_wasm)
                .collect::<Box<[_]>>()
                .iter()
                .copied()
                .rev()
                .collect_vec(),
            returns.clone(),
        )?;

        wasm.push(InternalInstruction::Immediate(WInstruction::If(
            BlockType::FunctionType(block_type),
        )));

        self.b_if.collapse_with_returns(func, wasm, returns)?;
        wasm.push(InternalInstruction::Immediate(WInstruction::Else));

        if let Some(b_else) = &self.b_else {
            b_else.collapse_with_returns(func, wasm, returns)?;
        } else {
            wasm.push(InternalInstruction::Immediate(WInstruction::Unreachable));
        }

        wasm.push(InternalInstruction::Immediate(WInstruction::End));

        Ok(())
    }

    /// Recursively collapse the branch into a list of instructions
    fn collapse(&self, func: &StepFunc, wasm: &mut Vec<InternalInstruction>) -> HQResult<()> {
        self.collapse_with_returns(func, wasm, &vec![])
    }
}

/// A block of instructions, possibly ending in a branch, along with information
/// about the type stack at the beginning of the block, and just before the branch
#[derive(Debug, Clone)]
struct Fork {
    body: RefCell<Vec<InternalInstruction>>,
    branch: RefCell<Option<Branch>>,
    /// the type stack at the start of the body
    entry: Rc<TypeStack>,
    /// the type stack at the end of the body, just before any branches
    exit: RefCell<Rc<TypeStack>>,
}

impl Fork {
    fn new(type_stack: &Rc<TypeStack>) -> Rc<Self> {
        Rc::new(Self {
            body: RefCell::new(vec![]),
            branch: RefCell::new(None),
            entry: Rc::clone(type_stack),
            exit: RefCell::new(Rc::clone(type_stack)),
        })
    }

    /// Recursively collapse the fork into a list of instructions
    fn collapse(&self, func: &StepFunc, wasm: &mut Vec<InternalInstruction>) -> HQResult<()> {
        self.collapse_with_returns(func, wasm, &vec![])
    }

    fn collapse_with_returns(
        &self,
        func: &StepFunc,
        wasm: &mut Vec<InternalInstruction>,
        returns: &Vec<ValType>,
    ) -> HQResult<()> {
        wasm.extend(self.body.try_borrow()?.iter().cloned());

        if let Some(branch) = &*self.branch.try_borrow()? {
            branch.collapse_with_returns(func, wasm, returns)?;
        }

        Ok(())
    }
}

/// Returns the NaN-box bit pattern for the provided type (which must be basic)
fn boxed_pattern(ty: IrType) -> HQResult<i64> {
    Ok(match ty {
        IrType::Int => BOXED_INT_PATTERN,
        IrType::Boolean => BOXED_BOOL_PATTERN,
        IrType::String => BOXED_STRING_PATTERN,
        IrType::ColorARGB => BOXED_COLOR_ARGB_PATTERN,
        IrType::ColorRGB => BOXED_COLOR_RGB_PATTERN,
        _ => hq_bug!("bad type for boxed pattern"),
    })
}

/// Returns the list of instructions used to unbox a boxed value of the provided (base) type
fn unbox_instructions(ty: IrType, func: &StepFunc) -> HQResult<Vec<InternalInstruction>> {
    Ok(match ty {
        IrType::Int | IrType::Boolean | IrType::ColorARGB | IrType::ColorRGB => wasm![I32WrapI64],
        IrType::String => {
            let table_index = func.registries().tables().register::<StringsTable, _>()?;
            wasm![I32WrapI64, TableGet(table_index)]
        }
        IrType::Float => wasm![F64ReinterpretI64],
        _ => hq_bug!("bad type for unboxing instructions"),
    })
}

/// Returns the list of instructions used to check if a boxed value is of the specified (base) type
fn box_type_check(ty: IrType) -> HQResult<Vec<InternalInstruction>> {
    Ok(if ty == IrType::Float {
        // float always comes last, and any i64 is always valid to be reinterpreted
        // as an f64, so return true for these. The redundant branch will be
        // optimised away by wasm-opt.
        wasm![Drop, I32Const(1)]
    } else {
        let box_pattern = boxed_pattern(ty)?;
        wasm![I64Const(box_pattern), I64And, I64Const(box_pattern), I64Eq,]
    })
}

type BoxedLocals = (Box<[IrType]>, Box<[u32]>);

fn build_boxed_locals(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<BoxedLocals> {
    let boxed_onward: Box<[_]> = inputs
        .iter()
        .copied()
        .skip_while(|ty| ty.is_base_type())
        .collect();
    let locals = boxed_onward
        .iter()
        .copied()
        .map(WasmProject::ir_type_to_wasm)
        .map(|w_ty| func.local(w_ty))
        .collect::<HQResult<Box<[_]>>>()?;
    Ok((boxed_onward, locals))
}

fn make_leaf(func: &StepFunc, leaf: &Rc<Fork>, new_ty: IrType, local: u32) -> HQResult<Rc<Fork>> {
    let base_ty = new_ty
        .base_type()
        .ok_or_else(|| make_hq_bug!("got non-base-type `new_ty`"))?;
    let leaf = Fork::new(&*leaf.exit.try_borrow()?);
    leaf.exit.try_borrow_mut()?.push_mut(new_ty);
    let mut body = leaf.body.try_borrow_mut()?;
    body.push(InternalInstruction::Immediate(WInstruction::LocalGet(
        local,
    )));
    body.extend(unbox_instructions(base_ty, func)?);
    Ok(Rc::clone(&leaf))
}

/// Makes a new branch for type selection, using the `latest_fork` as the else
/// branch, and a new leaf as the if branch. Adds the new leaf to `new_leaves`.
///
/// Precondition: `new_ty.is_base_type()`
///
/// [`{$I_b$}`](Branch#invariant-i_b) maintained because every new leaf in this set of branches has
/// the same entry.
fn make_branch(
    new_ty: IrType,
    local: u32,
    latest_fork: Option<&Rc<Fork>>,
    new_leaf: &Rc<Fork>,
) -> HQResult<Branch> {
    let branch = Branch {
        // TODO: condition
        condition: wasm![LocalGet(local),]
            .into_iter()
            .chain(box_type_check(
                #[expect(clippy::unwrap_used, reason = "guaranteed `Some` by precondition")]
                new_ty.base_type().unwrap(),
            )?)
            .collect(),
        b_if: Rc::clone(new_leaf),
        b_else: latest_fork.cloned(),
    };
    Ok(branch)
}

/// Precondition: `new_ty.is_base_type()`
/// Postcondition: `latest_fork.is_some()`
fn make_branch_fork(
    leaf: &Rc<Fork>,
    new_ty: IrType,
    local: u32,
    latest_fork: &mut Option<Rc<Fork>>,
    new_leaf: &Rc<Fork>,
) -> HQResult<()> {
    let branch = make_branch(new_ty, local, latest_fork.as_ref(), new_leaf)?;
    let fork = Fork::new(&Rc::clone(&*leaf.exit.try_borrow()?));
    *fork.branch.try_borrow_mut()? = Some(branch);
    *latest_fork = Some(fork);
    Ok(())
}

/// Postcondition: `new_leaves.is_some()`
fn make_type_branches(
    func: &StepFunc,
    leaf: &Rc<Fork>,
    input_ty: IrType,
    base_tys: &[IrType],
    local: u32,
    new_leaves: &mut Option<Leaves>,
) -> HQResult<()> {
    let mut latest_fork = None;
    let mut these_new_leaves =
        Leaves::new(make_leaf(func, leaf, base_tys[0].and(input_ty), local)?);
    for base_ty in base_tys.iter().skip(1).rev() {
        these_new_leaves.push_leaf(make_leaf(func, leaf, base_ty.and(input_ty), local)?);
    }
    for (new_leaf, base_ty) in (&these_new_leaves).into_iter().zip(base_tys).skip(1).rev() {
        make_branch_fork(
            leaf,
            // precondition satisfied because `and` is nonincreasing
            base_ty.and(input_ty),
            local,
            &mut latest_fork,
            new_leaf,
        )?;
    }
    let branch = make_branch(
        // precondition satisfied because `and` is nonincreasing
        base_tys[0].and(input_ty),
        local,
        latest_fork.as_ref(),
        these_new_leaves.first(),
    )?;
    *leaf.branch.try_borrow_mut()? = Some(branch);
    // `{$I_l$}` maintained because every new `Fork` in `these_new_leaves`
    // has exactly one more type on the stack than the `exit` of
    // the parent; inductively, `{$I_f'$}` holds for `new_leaves`.
    // `{$I_f'$}` evident from creation of new leaves.
    if let Some(new_leaves) = new_leaves {
        new_leaves.extend(these_new_leaves);
    } else {
        *new_leaves = Some(these_new_leaves);
    }

    Ok(())
}

fn request_screen_refresh(
    func: &StepFunc,
    opcode: &IrOpcode,
    mut body: RefMut<'_, Vec<InternalInstruction>>,
) -> HQResult<()> {
    if opcode.requests_screen_refresh() {
        let refresh_requested = func.registries().globals().register(
            "requests_refresh".into(),
            (
                ValType::I32,
                ConstExpr::i32_const(0),
                GlobalMutable(true),
                GlobalExportable(true),
            ),
        )?;

        body.append(&mut wasm![I32Const(1), #LazyGlobalSet(refresh_requested),]);
    }
    Ok(())
}

/// Should arguments to this opcode be unboxed?
const fn should_unbox(opcode: &IrOpcode) -> bool {
    // we don't want to unbox inputs to procedures, because... reasons,
    // and we don't want to unbox inputs to `dup` or `swap` either because
    // these are lower level instructions and don't really care about types
    !matches!(
        opcode,
        &IrOpcode::procedures_call_warp(_) | &IrOpcode::hq_dup | &IrOpcode::hq_swap
    )
}

/// Base, top-level branches across the whole function; i.e. branches that begin
/// with an empty type stack. Invariants `{$I_r$}` and `{$I_r'$}` declared here.
///
/// # Invariant `{$I_r$}`
/// ```typm-render
/// forall #`root` in #`roots`;[1..#`roots`.#`len`;()) . #`root`.#`entry`.#`is_nil`;().
/// ```
///
/// # Invariant `{$I_r'$}`
/// ```typm-render
///   forall #`root` in #`roots`[0..(#`roots`.#`len`() - 1)) .\
///     forall #`leaf` in L(#`root`) . #`leaf`.#`exit`.#`is_nil`(),
/// ```
/// i.e. all except the last root must have all leaves with an empty type stack.
struct Roots(Vec<Rc<Fork>>);

impl Roots {
    const fn new() -> Self {
        Self(vec![])
    }

    /// Appends a root to the end of the sequence.
    ///
    /// Precondition: `is_root(root)`
    fn push_root(&mut self, root: Rc<Fork>) {
        self.0.push(root);
    }
}

impl IntoIterator for Roots {
    type Item = Rc<Fork>;
    type IntoIter = alloc::vec::IntoIter<Rc<Fork>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// A collection of forks that are leaves. Invariant `{$I_l$}` declared here.
///
/// # Invariant `{$I_l$}`
/// ```typm-render
/// |{ #`leaf`.#`exit`.#`len`() | #`leaf` in #`leaves` }| = 1,
/// ```
/// i.e. all leaves have the same number of types on the stack.
///
/// # Invariant `{$I_l'$}`
/// `!leaves.0.is_empty()`
///
/// This is maintained because all mutating methods either initialise the collection
/// with a single leaf, or increase the length.
///
/// The invariant that all members of this sequence _are_ leaves is implied
/// by invariant [`{$I_f'$}`](ForestBuilderState#invariant-i_f-1).
struct Leaves(Vec<Rc<Fork>>);

impl Leaves {
    /// Creates a new collection of leaves, with one leaf to initialise.
    ///
    /// Precondition: `leaf.branch.is_none()`.
    fn new(leaf: Rc<Fork>) -> Self {
        Self(vec![leaf])
    }

    /// Reinitialises the collection with a single leaf.
    ///
    /// Precondition: `leaf.branch.is_none()`
    fn reinit(&mut self, leaf: Rc<Fork>) {
        self.0.clear();
        self.push_leaf(leaf);
    }

    /// Appends a leaf to the end of the collection.
    ///
    /// Precondition: `leaf.branch.is_none()`
    fn push_leaf(&mut self, leaf: Rc<Fork>) {
        self.0.push(leaf);
    }

    /// Returns the first leaf in the collection.
    ///
    /// This is guaranteed to exist because .
    fn first(&self) -> &Rc<Fork> {
        &self.0[0]
    }

    /// The number of leaves in the collection.
    const fn size(&self) -> usize {
        self.0.len()
    }

    /// Replaces this collection of leaves with some other collection of leaves.
    fn replace_with(&mut self, other: Self) {
        self.0 = other.0;
    }

    /// Appends another collection of leaves onto this one
    fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
    }
}

impl<'a> IntoIterator for &'a Leaves {
    type Item = &'a Rc<Fork>;
    type IntoIter = core::slice::Iter<'a, Rc<Fork>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// State during [`build_forest`]. Invariants `{$I_f$}` and `{$I_f'$}` declared here.
///
/// # Invariant `{$I_f$}`
/// ```typm-render
/// #`roots`;[#`roots`.#`len`;() - 1] = #`this_root`.
/// ```
///
/// # Invariant `{$I_f'$}`
/// ```typm-render
/// #`leaves` = L(#`this_root`).
/// ```
struct ForestBuilderState {
    this_root: Rc<Fork>,
    roots: Roots,
    leaves: Leaves,
}

impl ForestBuilderState {
    /// Adds a new root if the exit type stack on the latest tree is empty.
    ///
    /// Maintains invariant [`{$I_r$}`](Roots#invariant-i_r): a new root can
    /// only be added if the previous root has all leaves with empty type stacks.
    /// Inductively [`{$I_r$}`](Roots#invariant-i_r) must hold for any value of `roots`.
    fn add_new_root_if_needed(&mut self) {
        // By `{$I_l$}`, we only need to check if one leaf has an empty exit type stack.
        // leaves[0] is guaranteed to exist by `{$I_f'$}`; since the tree structure is
        // finite, there must exist at least one leaf.
        if self
            .leaves
            .first()
            .exit
            .try_borrow()
            .is_ok_and(|ty| ty.is_nil())
        {
            // Maintains `{$I_r'$}`
            self.this_root = Fork::new(&Rc::new(TypeStack::Nil));
            // Maintains `{$I_f$}`
            self.roots.push_root(Rc::clone(&self.this_root));
            // Maintains `{$I_f'$}`; this root is its own leaf. `{$I_l$}` trivial.
            self.leaves.reinit(Rc::clone(&self.this_root));
        }
    }
}

impl ForestBuilderState {
    fn new(type_stack: Rc<[IrType]>) -> Self {
        let this_root = Fork::new(&type_stack.iter().copied().collect());
        let mut roots = Roots::new();
        // Initialise to one empty branch so that `{$I_r$}` and `{$I_r'$}` hold.
        roots.push_root(Rc::clone(&this_root));
        // $ L(`#this_root`) = { #`this_root` } $
        let leaves = Leaves::new(Rc::clone(&this_root));
        Self {
            this_root,
            roots,
            leaves,
        }
    }
}

/// If all of the leaves have the same exit stack, we can pre-emptively merge
/// them together to avoid a huge blow up in the number of leaves.
/// 
/// Precondition: $ #`leaves` = L(#`ancestor`) $
fn collapse_same_typed_leaves(func: &StepFunc, ancestor: Rc<Fork>, leaves: &mut Leaves) -> HQResult<()> {
    if leaves
        .0
        .iter()
        .map(|new_leaf| Rc::clone(&*new_leaf.exit.borrow()))
        .all_equal()
    {
        let mut collapsed = wasm![];
        let same_exit = { Rc::clone(&*leaves.first().exit.try_borrow()?) };
        ancestor.collapse_with_returns(
            func,
            &mut collapsed,
            &same_exit
                .clone()
                .map(WasmProject::ir_type_to_wasm)
                .collect::<Box<[_]>>()
                .iter()
                .copied()
                .rev()
                .collect_vec(),
        )?;
        *ancestor.body.try_borrow_mut()? = collapsed;
        *ancestor.branch.try_borrow_mut()? = None;
        *ancestor.exit.try_borrow_mut()? = same_exit;
        leaves.reinit(Rc::clone(&ancestor));
    }
    Ok(())
}

/// Builds a 'forest' (collection of trees, in this case ordered) representing
/// the WASM for the specified opcodes, with the entry point having the specified
/// type stack.
///
/// Precondition: the specified type stack and opcodes are well-formed, i.e. all
/// inputs are valid for all opcodes at all points.
///
/// Postcondition: returns `roots` s.t.
/// ```typm-render
///   forall &#`root` in #`roots` . #`is_root`;(#`root`) \
///   and forall &#`root` in #`roots`;lr([0 .. (#`roots`.#`len`;() - 1))) space .\
///       &forall #`leaf` in L(#`root`) . #`leaf`.#`exit`.#`is_nil`;().
/// ```
fn build_forest(
    func: &StepFunc,
    type_stack: Rc<[IrType]>,
    opcodes: &[IrOpcode],
) -> HQResult<Roots> {
    let mut state = ForestBuilderState::new(type_stack);

    for opcode in opcodes {
        state.add_new_root_if_needed();

        let expected_inputs = opcode.acceptable_inputs()?;

        let mut new_leaves: Option<Leaves> = None;

        for leaf in &state.leaves {
            let actual_inputs = {
                leaf.exit
                    .try_borrow()?
                    .clone()
                    .take(expected_inputs.len())
                    .collect::<Box<[_]>>()
                    .iter()
                    .copied()
                    .rev()
                    .collect::<Rc<[_]>>()
            };

            hq_assert_eq!(actual_inputs.len(), expected_inputs.len());

            // All the inputs past the first boxed input (possibly empty)
            let (boxed_onward, locals) = if should_unbox(opcode) {
                build_boxed_locals(func, Rc::clone(&actual_inputs))?
            } else {
                (Box::from(&[] as &[_]), Box::from(&[] as &[_]))
            };

            // Block necessary here so that mutable references are dropped
            {
                // Store all inputs, from the first boxed one, in locals
                leaf.body.try_borrow_mut()?.extend(
                    locals
                        .iter()
                        .rev()
                        .copied()
                        .map(WInstruction::LocalSet)
                        .map(InternalInstruction::Immediate),
                );

                let mut exit = leaf.exit.try_borrow_mut()?;
                // `{$I_l$}` maintained because all leaves have exactly one element dropped
                // from the type stack
                exit.drop_mut(boxed_onward.len());
            }

            let mut these_new_leaves = Leaves::new(Rc::clone(leaf));

            for (input_ty, local) in boxed_onward.iter().zip(locals) {
                if input_ty.is_base_type() {
                    for leaf_leaf in &these_new_leaves {
                        leaf_leaf
                            .body
                            .try_borrow_mut()?
                            .push(InternalInstruction::Immediate(WInstruction::LocalGet(
                                local,
                            )));
                        // `{$I_l$}` maintained because exactly one element pushed for every leaf
                        leaf_leaf.exit.try_borrow_mut()?.push_mut(*input_ty);
                    }
                } else {
                    let base_tys: Box<[_]> = input_ty.base_types().collect();
                    let mut these_these_new_leaves = None;
                    for leaf_leaf in &these_new_leaves {
                        make_type_branches(
                            func,
                            leaf_leaf,
                            *input_ty,
                            &base_tys,
                            local,
                            &mut these_these_new_leaves,
                        )?;
                    }

                    these_new_leaves = #[expect(
                        clippy::unwrap_used,
                        reason = "postcondition of `make_type_branches` guarantees that this is \
                                  Some"
                    )]
                    these_these_new_leaves.unwrap();
                }
                func.free_local(local)?;
            }

            for leaf_leaf in &these_new_leaves {
                let mut exit = leaf_leaf.exit.try_borrow_mut()?;
                // Remove top elements from stack. Maintains `{$I_l$}` because same action
                // taken for every leaf.
                let this_inputs: Rc<[_]> = exit
                    .take_n(expected_inputs.len())
                    .into_iter()
                    .rev()
                    .collect();
                if this_inputs.len() != expected_inputs.len() {
                    hq_bug!("not enough types on stack!");
                }

                let mut body = leaf_leaf.body.try_borrow_mut()?;
                body.extend(opcode.wasm(func, Rc::clone(&this_inputs))?);

                request_screen_refresh(func, opcode, body)?;

                match opcode.output_type(this_inputs)? {
                    ReturnType::None => (),
                    ReturnType::Singleton(ty) => exit.push_mut(ty),
                    ReturnType::MultiValue(tys) => {
                        for ty in tys.iter() {
                            exit.push_mut(*ty);
                        }
                    }
                }
            }

            collapse_same_typed_leaves(func, Rc::clone(leaf), &mut these_new_leaves)?;

            if let Some(new_leaves) = new_leaves.as_mut() {
                new_leaves.extend(these_new_leaves);
            } else {
                new_leaves = Some(these_new_leaves);
            }
        }

        state.leaves = #[expect(
            clippy::unwrap_used,
            reason = "`Leaves` always nonempty, so loop always runs at east once, so `new_leaves` \
                      is definitely initialised to a `Some` value"
        )]
        new_leaves.unwrap();

        collapse_same_typed_leaves(func, Rc::clone(&state.this_root), &mut state.leaves)?;
    }

    Ok(state.roots)
}

pub fn wrap_instructions(
    func: &StepFunc,
    type_stack: Rc<[IrType]>,
    opcodes: &[IrOpcode],
) -> HQResult<Vec<InternalInstruction>> {
    let roots = build_forest(func, type_stack, opcodes)?;

    let mut wasm = vec![];
    for root in roots {
        root.collapse(func, &mut wasm)?;
    }

    Ok(wasm)
}
