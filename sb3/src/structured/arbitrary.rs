use core::marker::PhantomData;
use std::ops::ControlFlow;

pub use ::arbitrary::{Arbitrary, MaxRecursionReached, Result, Unstructured, details, size_hint};

use super::*;

pub trait ArbitraryContext {}

macro_rules! ac {
    ($($ty:ty),+ $(,)?) => {
        $(impl<'a, Ctx: ArbitraryContext> ArbitraryWithContext<'a, Ctx> for $ty {
            fn arbitrary_with_context(u: &mut Unstructured<'a>, _context: &Ctx) -> arbitrary::Result<Self> {
                u.arbitrary()
            }
        })+
    };
}

ac!(
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
    bool,
    char,
    (),
    &'a str,
    String,
    Box<str>
);

#[derive(Clone)]
pub struct ProjectContext {
    pub global_variables: Registry<GlobalVariableId, Variable>,
    pub global_lists: Registry<GlobalListId, List>,
    pub broadcasts: Registry<BroadcastId, Broadcast>,
    pub num_targets: usize,
}

impl ArbitraryContext for ProjectContext {}

#[derive(Clone)]
pub struct TargetContext {
    pub global_variables: Registry<GlobalVariableId, Variable>,
    pub global_lists: Registry<GlobalListId, List>,
    pub local_variables: Registry<LocalVariableId, Variable>,
    pub local_lists: Registry<LocalListId, List>,
    pub broadcasts: Registry<BroadcastId, Broadcast>,
    pub procedures: Registry<ProcedureId, Procedure>,
    pub num_targets: usize,
}

impl ArbitraryContext for TargetContext {}

impl From<TargetContext> for ProjectContext {
    fn from(
        TargetContext {
            broadcasts,
            global_lists,
            global_variables,
            num_targets,
            ..
        }: TargetContext,
    ) -> Self {
        Self {
            broadcasts,
            global_lists,
            global_variables,
            num_targets,
        }
    }
}

pub trait ArbitraryWithContext<'a, Ctx: ArbitraryContext>: Sized {
    fn arbitrary_with_context(u: &mut Unstructured<'a>, context: &Ctx) -> arbitrary::Result<Self>;
}

struct ArbitraryContextIter<'a, 'b, 'c, Ctx: ArbitraryContext, ElementType> {
    u: &'b mut Unstructured<'a>,
    ctx: &'c Ctx,
    _marker: PhantomData<ElementType>,
}

impl<'a, Ctx: ArbitraryContext, ElementType: ArbitraryWithContext<'a, Ctx>> Iterator
    for ArbitraryContextIter<'a, '_, '_, Ctx, ElementType>
{
    type Item = Result<ElementType>;
    fn next(&mut self) -> Option<Result<ElementType>> {
        let keep_going = self.u.arbitrary().unwrap_or(false);
        if keep_going {
            Some(ArbitraryWithContext::arbitrary_with_context(
                self.u, self.ctx,
            ))
        } else {
            None
        }
    }
}

trait UnstructuredWithContext<'a, Ctx: ArbitraryContext> {
    fn arbitrary_iter_ctx<'b, 'c, ElementType: ArbitraryWithContext<'a, Ctx>>(
        &'b mut self,
        context: &'c Ctx,
    ) -> Result<ArbitraryContextIter<'a, 'b, 'c, Ctx, ElementType>>;
}

impl<'a, Ctx: ArbitraryContext> UnstructuredWithContext<'a, Ctx> for Unstructured<'a> {
    fn arbitrary_iter_ctx<'b, 'c, ElementType: ArbitraryWithContext<'a, Ctx>>(
        &'b mut self,
        context: &'c Ctx,
    ) -> Result<ArbitraryContextIter<'a, 'b, 'c, Ctx, ElementType>> {
        Ok(ArbitraryContextIter {
            u: &mut *self,
            ctx: context,
            _marker: PhantomData,
        })
    }
}

impl<'a, Ctx: ArbitraryContext, A: ArbitraryWithContext<'a, Ctx>> ArbitraryWithContext<'a, Ctx>
    for Option<A>
{
    fn arbitrary_with_context(u: &mut Unstructured<'a>, context: &Ctx) -> arbitrary::Result<Self> {
        if u.ratio(1, 8)? {
            Ok(None)
        } else {
            Ok(Some(ArbitraryWithContext::arbitrary_with_context(
                u, context,
            )?))
        }
    }
}

impl<'a, Ctx: ArbitraryContext, A: ArbitraryWithContext<'a, Ctx>> ArbitraryWithContext<'a, Ctx>
    for Box<A>
where
    Ctx: Into<Ctx>,
{
    fn arbitrary_with_context(u: &mut Unstructured<'a>, context: &Ctx) -> arbitrary::Result<Self> {
        Ok(Box::new(ArbitraryWithContext::arbitrary_with_context(
            u, context,
        )?))
    }
}

impl<'a, Ctx: ArbitraryContext, A: ArbitraryWithContext<'a, Ctx>> ArbitraryWithContext<'a, Ctx>
    for Vec<A>
where
    Ctx: Into<Ctx>,
{
    fn arbitrary_with_context(u: &mut Unstructured<'a>, context: &Ctx) -> arbitrary::Result<Self> {
        u.arbitrary_iter_ctx(context)?.collect()
    }
}

macro_rules! arbitrary_context {
    (@arbitrary_func $context:ident @ $($expr:expr;)*) => {
        fn arbitrary_with_context(u: &mut Unstructured<'a>, context: &$context) -> arbitrary::Result<Self> {
            #[allow(unused)]
            let choices = [
                $($expr),*
            ];
            u.choose(&choices)?(u, context)
        }
    };

    // base case
    ($context:ident @ $pub:vis enum $id:ident
        (@variants $($tt:tt)*)
        (@builders $($builder:expr;)*)
    ) => {
        $pub enum $id {
            $($tt)*
        }

        impl<'a> ArbitraryWithContext<'a, $context> for $id {
            arbitrary_context!(@arbitrary_func $context @ $($builder;)*);
        }
    };
    // recursive case: unit variant
    ($context:ident @ $pub:vis enum $id:ident
        (@variants $($tt:tt)*)
        (@builders $($builder:expr;)*)
        $unit:ident,
        $($rest:tt)*
    ) => {
        arbitrary_context!(
            $context @ $pub enum $id
            (@variants $($tt)* $unit,)
            (@builders
                $($builder;)*
                |u: &mut _, context| Ok($id::$unit);
            )
            $($rest)*
        );
    };
    ($context:ident @ $pub:vis enum $id:ident
        (@variants $($tt:tt)*)
        (@builders $($builder:expr;)*)
        #[arbitrary(skip)]
        $unit:ident,
        $($rest:tt)*
    ) => {
        arbitrary_context!(
            $context @ $pub enum $id
            (@variants $($tt)* $unit,)
            (@builders $($builder;)*)
            $($rest)*
        );
    };

    // recursive case: tuple variant
    ($context:ident @ $pub:vis enum $id:ident
        (@variants $($tt:tt)*)
        (@builders $($builder:expr;)*)
        $name:ident($($ty:ty),*),
        $($rest:tt)*
    ) => {
        arbitrary_context!(
            $context @ $pub enum $id
            (@variants $($tt)* $name($($ty),*),)
            (@builders
                $($builder;)*
                |u: &mut _, context| Ok($id::$name($(<$ty as ArbitraryWithContext<'a, $context>>::arbitrary_with_context(u, context)?),*));
            )
            $($rest)*
        );
    };
    ($context:ident @ $pub:vis enum $id:ident
        (@variants $($tt:tt)*)
        (@builders $($builder:expr;)*)
        #[arbitrary(skip)]
        $name:ident($($ty:ty),*),
        $($rest:tt)*
    ) => {
        arbitrary_context!(
            $context @ $pub enum $id
            (@variants $($tt)* $name($($ty),*),)
            (@builders $($builder;)*)
            $($rest)*
        );
    };

    // recursive case: struct variant
    ($context:ident @ $pub:vis enum $id:ident
        (@variants $($tt:tt)*)
        (@builders $($builder:expr;)*)
        $name:ident { $($field:ident: $ty:ty),* $(,)? },
        $($rest:tt)*
    ) => {
        arbitrary_context!(
            $context @ $pub enum $id
            (@variants $($tt)* $name { $($field: $ty),* },)
            (@builders
                $($builder;)*
                |u: &mut _, context| Ok($id::$name {
                    $($field: <$ty as ArbitraryWithContext<'a, $context>>::arbitrary_with_context(u, context)?),*
                });
            )
            $($rest)*
        );
    };
    ($context:ident @ $pub:vis enum $id:ident
        (@variants $($tt:tt)*)
        (@builders $($builder:expr;)*)
        #[arbitrary(skip)]
        $name:ident { $($field:ident: $ty:ty),* $(,)? },
        $($rest:tt)*
    ) => {
        arbitrary_context!(
            $context @ $pub enum $id
            (@variants $($tt)* $name { $($field: $ty),* },)
            (@builders $($builder;)*)
            $($rest)*
        );
    };

    // entry points
    attr() ($item:item) => {
        #[arbitrary_context(TargetContext)]
        $item
    };
    attr($context:ident) ($pub:vis enum $id:ident {
        $($tt:tt)+
    }) => {
        arbitrary_context!(
            $context @ $pub enum $id
            (@variants)
            (@builders)
            $($tt)+
        );
    };

    attr($context:ident) ($pub:vis struct $id:ident {
        $($fieldpub:vis $field:ident: $ty:ty),+ $(,)?
    }) => {
        $pub struct $id {
            $($fieldpub $field: $ty),+
        }

        impl<'a> ArbitraryWithContext<'a, $context> for $id {
            fn arbitrary_with_context(u: &mut Unstructured<'a>, context: &$context) -> arbitrary::Result<Self> {
                Ok(Self {
                    $($field: <$ty as ArbitraryWithContext<'a, $context>>::arbitrary_with_context(u, context)?),+
                })
            }
        }
    };
}

pub(super) use arbitrary_context;

impl<'a> Arbitrary<'a> for StructuredProject {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let global_variables: Registry<_, _> =
            u.arbitrary_iter::<Variable>()?.collect::<Result<_>>()?;
        let global_lists: Registry<_, _> = u.arbitrary_iter::<List>()?.collect::<Result<_>>()?;
        let broadcasts: Registry<_, _> = u.arbitrary_iter::<Broadcast>()?.collect::<Result<_>>()?;

        let mut project_context = ProjectContext {
            global_lists: global_lists.clone(),
            global_variables: global_variables.clone(),
            broadcasts: broadcasts.clone(),
            num_targets: 0,
        };

        let mut targets: Vec<StructuredTarget> = Vec::new();
        u.arbitrary_loop(None, Some(6), |u| {
            targets.push(ArbitraryWithContext::arbitrary_with_context(
                u,
                &project_context,
            )?);
            project_context.num_targets += 1;
            Ok(ControlFlow::Continue(()))
        })?;

        Ok(Self {
            global_variables,
            global_lists,
            broadcasts,
            targets,
            monitors: Arbitrary::arbitrary(u)?,
            extensions: vec![], // TODO
            meta: Meta {
                semver: "3.0.0".into(),
                vm: "3.0.0".into(),
                agent: "hyperquark-fuzzer".into(),
            },
        })
    }
}

impl<'a> ArbitraryWithContext<'a, ProjectContext> for StructuredTarget {
    fn arbitrary_with_context(
        u: &mut Unstructured<'a>,
        ProjectContext {
            global_variables,
            global_lists,
            broadcasts,
            num_targets,
        }: &ProjectContext,
    ) -> arbitrary::Result<Self> {
        let mut local_variables = Registry::new();
        let mut local_lists = Registry::new();
        if *num_targets > 0 {
            local_variables = u.arbitrary_iter::<Variable>()?.collect::<Result<_>>()?;
            local_lists = u.arbitrary_iter::<List>()?.collect::<Result<_>>()?;
        }

        let local_procedures: Registry<_, _> =
            u.arbitrary_iter::<Procedure>()?.collect::<Result<_>>()?;

        let target_context = TargetContext {
            global_variables: global_variables.clone(),
            global_lists: global_lists.clone(),
            broadcasts: broadcasts.clone(),
            num_targets: *num_targets,
            local_variables: local_variables.clone(),
            local_lists: local_lists.clone(),
            procedures: local_procedures.clone(),
        };

        let mut scripts = Vec::with_capacity(local_procedures.items.len());
        for procedure in 0..local_procedures.items.len() {
            let procedure = ProcedureId(procedure);
            scripts.push(Script {
                hat: Some(Hat::ProcedureDefinition { procedure }),
                position: Arbitrary::arbitrary(u)?,
                body: ArbitraryWithContext::arbitrary_with_context(u, &target_context)?,
                top_reporter: None,
                // TODO: use procedure context instead
            });
        }
        scripts.extend(
            u.arbitrary_iter_ctx(&target_context)?
                .collect::<Result<Box<[_]>>>()?,
        );

        Ok(StructuredTarget {
            is_stage: *num_targets == 0,
            name: if *num_targets == 0 {
                "Stage".into()
            } else {
                Arbitrary::arbitrary(u)?
            },
            local_variables,
            local_lists,
            local_procedures,
            scripts,
            comments: Vec::new(),
            current_costume: Arbitrary::arbitrary(u)?,
            costumes: Arbitrary::arbitrary(u)?,
            sounds: Arbitrary::arbitrary(u)?,
            layer_order: Arbitrary::arbitrary(u)?,
            volume: Arbitrary::arbitrary(u)?,
            tempo: Arbitrary::arbitrary(u)?,
            video_state: Arbitrary::arbitrary(u)?,
            video_transparency: Arbitrary::arbitrary(u)?,
            text_to_speech_language: Arbitrary::arbitrary(u)?,
            visible: Arbitrary::arbitrary(u)?,
            x: Arbitrary::arbitrary(u)?,
            y: Arbitrary::arbitrary(u)?,
            size: Arbitrary::arbitrary(u)?,
            direction: Arbitrary::arbitrary(u)?,
            draggable: Arbitrary::arbitrary(u)?,
            rotation_style: Arbitrary::arbitrary(u)?,
        })
    }
}

macro_rules! subtype_arbitrary {
    ($sup:ident -> $sub:ident for $id:ident) => {
        impl<'a> ArbitraryWithContext<'a, $sub> for $id {
            fn arbitrary_with_context(
                u: &mut Unstructured<'a>,
                context: &$sub,
            ) -> arbitrary::Result<Self> {
                ArbitraryWithContext::<$sup>::arbitrary_with_context(u, &context.clone().into())
            }
        }
    };
}

impl<'a> ArbitraryWithContext<'a, ProjectContext> for BroadcastId {
    fn arbitrary_with_context(
        u: &mut Unstructured<'a>,
        context: &ProjectContext,
    ) -> arbitrary::Result<Self> {
        Ok(Self(u.choose_index(context.broadcasts.items.len())?))
    }
}
subtype_arbitrary!(ProjectContext -> TargetContext for BroadcastId);

impl<'a> ArbitraryWithContext<'a, ProjectContext> for GlobalVariableId {
    fn arbitrary_with_context(
        u: &mut Unstructured<'a>,
        context: &ProjectContext,
    ) -> arbitrary::Result<Self> {
        Ok(Self(u.choose_index(context.global_variables.items.len())?))
    }
}
subtype_arbitrary!(ProjectContext -> TargetContext for GlobalVariableId);

impl<'a> ArbitraryWithContext<'a, ProjectContext> for GlobalListId {
    fn arbitrary_with_context(
        u: &mut Unstructured<'a>,
        context: &ProjectContext,
    ) -> arbitrary::Result<Self> {
        Ok(Self(u.choose_index(context.global_lists.items.len())?))
    }
}
subtype_arbitrary!(ProjectContext -> TargetContext for GlobalListId);

impl<'a> ArbitraryWithContext<'a, TargetContext> for ProcedureId {
    fn arbitrary_with_context(
        u: &mut Unstructured<'a>,
        context: &TargetContext,
    ) -> arbitrary::Result<Self> {
        Ok(Self(u.choose_index(context.procedures.items.len())?))
    }
}

impl<'a> ArbitraryWithContext<'a, TargetContext> for LocalVariableId {
    fn arbitrary_with_context(
        u: &mut Unstructured<'a>,
        context: &TargetContext,
    ) -> arbitrary::Result<Self> {
        Ok(Self(u.choose_index(context.local_variables.items.len())?))
    }
}

impl<'a> ArbitraryWithContext<'a, TargetContext> for LocalListId {
    fn arbitrary_with_context(
        u: &mut Unstructured<'a>,
        context: &TargetContext,
    ) -> arbitrary::Result<Self> {
        Ok(Self(u.choose_index(context.local_lists.items.len())?))
    }
}
