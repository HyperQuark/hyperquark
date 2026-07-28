use mutatis::{Candidates, DefaultMutate, Mutate, mutators as ms};

use super::*;

/// A mutator that won't ever grow the input (other than for fields that use a default mutator)
#[derive(Default)]
pub struct StructuredShrinkMutator;

macro_rules! m {
    ($t:ty) => {
        impl DefaultMutate for $t {
            type DefaultMutate = StructuredShrinkMutator;
        }
    };
}

fn mutate_option<M, T>(
    m: &mut M,
    c: &mut Candidates<'_>,
    opt: &mut Option<T>,
) -> mutatis::Result<()>
where
    M: Mutate<T>,
{
    if opt.is_some() {
        c.mutation(|_ctx| {
            *opt = None;
            Ok(())
        })?;
        m.mutate(c, opt.as_mut().unwrap())?;
    }
    Ok(())
}

impl Mutate<Literal> for StructuredShrinkMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, lit: &mut Literal) -> mutatis::Result<()> {
        match lit {
            Literal::Number(n) => ms::f64().mutate(c, n)?,
            Literal::String(s) => {
                let mut st = s.to_string();
                ms::string(ms::char()).mutate(c, &mut st)?;
                c.mutation(|_ctx| {
                    *s = st.clone().into_boxed_str();
                    Ok(())
                })?
            }
            Literal::Color(_) => (),
        };
        Ok(())
    }
}
m! {Literal}

impl Mutate<Predicate> for StructuredShrinkMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, pred: &mut Predicate) -> mutatis::Result<()> {
        use Predicate as P;
        let boxed = &mut ms::boxed(self);
        let mut replacement_candidates = vec![];
        match pred {
            P::LessThan(left, right) | P::Equals(left, right) | P::GreaterThan(left, right) => {
                mutate_option(boxed, c, left)?;
                mutate_option(boxed, c, right)?;
                if let Some(l) = left
                    && let Value::Predicate(l) = *l.clone()
                {
                    replacement_candidates.push(*l.clone());
                }
                if let Some(r) = right
                    && let Value::Predicate(r) = *r.clone()
                {
                    replacement_candidates.push(*r.clone());
                }
            }
            P::And(left, right) | P::Or(left, right) => {
                mutate_option(boxed, c, left)?;
                mutate_option(boxed, c, right)?;
                if let Some(l) = left {
                    replacement_candidates.push(*l.clone());
                }
                if let Some(r) = right {
                    replacement_candidates.push(*r.clone());
                }
            }
            P::Not(input) => {
                mutate_option(boxed, c, input)?;
                if let Some(i) = input {
                    replacement_candidates.push(*i.clone());
                }
            }
            P::MouseDown | P::ProcedureArgument(_) => (),
            P::ListContainsItem { item: input, .. }
            | P::ItemOfList { index: input, .. }
            | P::ItemNumOfList { item: input, .. }
            | P::KeyPressed(input) => {
                mutate_option(boxed, c, input)?;
                if let Some(i) = input
                    && let Value::Predicate(i) = *i.clone()
                {
                    replacement_candidates.push(*i.clone());
                }
            }
        };
        if !replacement_candidates.is_empty() {
            c.mutation(|ctx| {
                let repl = ctx.rng().choose(&replacement_candidates).unwrap();
                *pred = repl.clone();
                Ok(())
            })?;
        }
        Ok(())
    }
}
m! {Predicate}

impl Mutate<Value> for StructuredShrinkMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, value: &mut Value) -> mutatis::Result<()> {
        use Value as V;
        let boxed = &mut ms::boxed(self);
        let mut replacement_candidates = vec![];
        match value {
            V::Literal(literal) => ms::default::<Literal>().mutate(c, literal)?,
            V::Predicate(pred) => boxed.mutate(c, pred)?,
            V::Add(left, right)
            | V::Subtract(left, right)
            | V::Multiply(left, right)
            | V::Divide(left, right)
            | V::Random(left, right)
            | V::Join(left, right)
            | V::LetterOf {
                letter: left,
                text: right,
            }
            | V::Contains {
                text: left,
                search: right,
            }
            | V::Modulo(left, right) => {
                mutate_option(boxed, c, left)?;
                mutate_option(boxed, c, right)?;
                if let Some(l) = left {
                    replacement_candidates.push(*l.clone());
                }
                if let Some(r) = right {
                    replacement_candidates.push(*r.clone());
                }
            }
            V::Length(input)
            | V::Round(input)
            | V::MathOp { operand: input, .. }
            | V::ItemOfList { index: input, .. }
            | V::ItemNumOfList { item: input, .. } => {
                mutate_option(boxed, c, input)?;
                if let Some(i) = input {
                    replacement_candidates.push(*i.clone());
                }
            }
            V::LengthOfList(_)
            | V::Answer
            | V::MouseX
            | V::MouseY
            | V::Timer
            | V::DaysSince2000
            | V::XPosition
            | V::YPosition
            | V::Direction
            | V::Size
            | V::CostumeNumber
            | V::BackdropNumber
            | V::Volume
            | V::Variable(_)
            | V::ListContents(_)
            | V::ProcedureArgument(_)
            | V::PenMenuColorParam(_)
            | V::KeyOptions(_) => (),
        }
        if !replacement_candidates.is_empty() {
            c.mutation(|ctx| {
                let repl = ctx.rng().choose(&replacement_candidates).unwrap();
                *value = repl.clone();
                Ok(())
            })?;
        }
        Ok(())
    }
}
m! {Value}

impl Mutate<ProcedureInput> for StructuredShrinkMutator {
    fn mutate(
        &mut self,
        c: &mut Candidates<'_>,
        input: &mut ProcedureInput,
    ) -> mutatis::Result<()> {
        match input {
            ProcedureInput::Value(value) => mutate_option(self, c, value)?,
            ProcedureInput::Predicate(pred) => mutate_option(self, c, pred)?,
        }
        Ok(())
    }
}
m! {ProcedureInput}

fn shrink_mutate_vec<M, T>(
    m: &mut M,
    c: &mut Candidates<'_>,
    vec: &mut Vec<T>,
) -> mutatis::Result<()>
where
    M: Mutate<T>,
{
    // Remove an element.
    if !vec.is_empty() {
        c.mutation(|ctx| {
            let index = ctx.rng().gen_index(vec.len()).unwrap();
            vec.remove(index);
            Ok(())
        })?;
    }

    // Mutate an existing element.
    for x in vec {
        m.mutate(c, x)?;
    }

    Ok(())
}

impl Mutate<Statement> for StructuredShrinkMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, stack: &mut Statement) -> mutatis::Result<()> {
        use Statement as S;
        match stack {
            S::SetVariable { value, .. }
            | S::ChangeVariable { value, .. }
            | S::Say { message: value }
            | S::SwitchCostumeTo { costume: value }
            | S::SwitchBackdropTo { backdrop: value }
            | S::SwitchBackdropToAndWait { backdrop: value }
            | S::ChangeSizeBy { amount: value }
            | S::SetSizeTo { size: value }
            | S::MoveSteps { steps: value }
            | S::AddToList { item: value, .. }
            | S::DeleteOfList { index: value, .. }
            | S::Wait { duration: value }
            | S::AskAndWait { question: value }
            | S::Think { message: value }
            | S::TurnRight { degrees: value }
            | S::TurnLeft { degrees: value }
            | S::PointInDirection { direction: value }
            | S::ChangeXBy { amount: value }
            | S::SetX { value }
            | S::ChangeYBy { amount: value }
            | S::SetY { value }
            | S::PenSetColorToColor { value }
            | S::PenSetSizeTo { value } => mutate_option(self, c, value)?,
            S::NextCostume
            | S::NextBackdrop
            | S::ShowVariable { .. }
            | S::HideVariable { .. }
            | S::DeleteAllOfList { .. }
            | S::ShowList { .. }
            | S::HideList { .. }
            | S::Stop { .. }
            | S::Broadcast { .. }
            | S::BroadcastAndWait { .. }
            | S::ResetTimer
            | S::Show
            | S::Hide
            | S::PenDown
            | S::PenUp
            | S::PenClear => (),
            S::InsertAtList {
                index: left,
                item: right,
                ..
            }
            | S::ReplaceItemOfList {
                index: left,
                item: right,
                ..
            }
            | S::SayForSecs {
                message: left,
                seconds: right,
            }
            | S::ThinkForSecs {
                message: left,
                seconds: right,
            }
            | S::GoToXY { x: left, y: right }
            | S::PenChangeColorParamBy {
                param: left,
                value: right,
            }
            | S::PenSetColorParamTo {
                param: left,
                value: right,
            } => {
                mutate_option(self, c, left)?;
                mutate_option(self, c, right)?;
            }
            S::WaitUntil { condition: pred } => mutate_option(self, c, pred)?,
            S::If {
                condition: pred,
                body: substack,
            }
            | S::RepeatUntil {
                condition: pred,
                body: substack,
            }
            | S::While {
                condition: pred,
                body: substack,
            } => {
                mutate_option(self, c, pred)?;
                shrink_mutate_vec(self, c, substack)?;
            }
            S::IfElse {
                condition: pred,
                then_body: substack1,
                else_body: substack2,
            } => {
                mutate_option(self, c, pred)?;
                shrink_mutate_vec(self, c, substack1)?;
                shrink_mutate_vec(self, c, substack2)?;
            }
            S::Repeat {
                times: value,
                body: substack,
            } => {
                mutate_option(self, c, value)?;
                shrink_mutate_vec(self, c, substack)?;
            }
            S::Forever { body: substack } => shrink_mutate_vec(self, c, substack)?,
            S::CallProcedure { arguments, .. } => {
                for arg in arguments {
                    self.mutate(c, arg)?;
                }
            }
        }
        Ok(())
    }
}
m! {Statement}

impl Mutate<Script> for StructuredShrinkMutator {
    fn mutate(&mut self, c: &mut Candidates<'_>, script: &mut Script) -> mutatis::Result<()> {
        shrink_mutate_vec(self, c, &mut script.body)
    }
}
m! {Script}

impl Mutate<StructuredTarget> for StructuredShrinkMutator {
    fn mutate(
        &mut self,
        c: &mut Candidates<'_>,
        target: &mut StructuredTarget,
    ) -> mutatis::Result<()> {
        shrink_mutate_vec(self, c, &mut target.scripts)?;

        for list in &mut target.local_lists.items {
            if !list.value.is_empty() {
                c.mutation(|ctx| {
                    let lower = ctx.rng().gen_index(list.value.len()).unwrap();
                    let upper = ctx
                        .rng()
                        .gen_index(list.value.len() - lower - 1)
                        .unwrap_or_default()
                        + lower
                        + 1;
                    list.value.splice(lower..upper, []);
                    Ok(())
                })?;

                shrink_mutate_vec(self, c, &mut target.scripts)?; // for balance
            }
        }
        Ok(())
    }
}
m! {StructuredTarget}

impl Mutate<StructuredProject> for StructuredShrinkMutator {
    fn mutate(
        &mut self,
        c: &mut Candidates<'_>,
        project: &mut StructuredProject,
    ) -> mutatis::Result<()> {
        let targets = &mut project.targets;
        if targets.len() > 1 {
            c.mutation(|ctx| {
                let mut index = ctx.rng().gen_index(targets.len()).unwrap();
                while targets[index].is_stage {
                    index = ctx.rng().gen_index(targets.len()).unwrap();
                }
                targets.remove(index);
                Ok(())
            })?;
        }

        for list in &mut project.global_lists.items {
            if !list.value.is_empty() {
                c.mutation(|ctx| {
                    let lower = ctx.rng().gen_index(list.value.len()).unwrap();
                    let upper = ctx
                        .rng()
                        .gen_index(list.value.len() - lower - 1)
                        .unwrap_or_default()
                        + lower
                        + 1;
                    list.value.splice(lower..upper, []);
                    Ok(())
                })?;
            }
        }

        // Mutate an existing element.
        for x in targets {
            self.mutate(c, x)?;
            self.mutate(c, x)?;
        }

        Ok(())
    }
}
m! {StructuredProject}
