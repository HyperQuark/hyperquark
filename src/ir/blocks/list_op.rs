use crate::instructions::{ControlIfElseFields, DataLengthoflistFields, DataSetvariabletoFields, DataVariableFields, HqCastFields, HqIntegerFields, HqTextFields, IrOpcode};
use crate::ir::{IrProject, RcList, RcVar, Step, StepContext, Type as IrType};
use crate::prelude::*;
use crate::sb3::VarVal;
use crate::wasm::WasmFlags;

pub fn generate_list_index_op<B>(
    list: &RcList,
    block: B,
    maybe_all_block: Option<IrOpcode>,
    other_argument: bool,
    add_one_to_length: bool,
    default_output: Option<&IrOpcode>,
    context: &StepContext,
    project: &Weak<IrProject>,
    flags: &WasmFlags,
) -> HQResult<Vec<IrOpcode>>
where
    B: Fn() -> IrOpcode,
{
    let text_var = RcVar::new(IrType::String, &VarVal::String("".into()), None, flags)?;
    let int_var = RcVar::new(IrType::Int, &VarVal::Int(0), None, flags)?;
    let extra_var = RcVar::new_empty();
    let result_var = RcVar::new_empty();

    let has_output = default_output.is_some();

    let result_step = |mut opcodes: Vec<IrOpcode>| {
        if has_output {
            opcodes.push(IrOpcode::data_setvariableto(DataSetvariabletoFields {
                var: RefCell::new(result_var.clone()),
                local_write: RefCell::new(true),
                first_write: RefCell::new(false),
            }));
        }
        Rc::new(RefCell::new(Step::new(
            None,
            context.clone(),
            opcodes,
            Weak::clone(project),
            false,
        )))
    };

    let int_step = result_step(if other_argument {
        vec![
            IrOpcode::data_variable(DataVariableFields {
                var: RefCell::new(int_var.clone()),
                local_read: RefCell::new(true),
            }),
            IrOpcode::hq_cast(HqCastFields(IrType::Int)),
            IrOpcode::data_variable(DataVariableFields {
                var: RefCell::new(extra_var.clone()),
                local_read: RefCell::new(true),
            }),
            block(),
        ]
    } else {
        vec![
            IrOpcode::data_variable(DataVariableFields {
                var: RefCell::new(int_var.clone()),
                local_read: RefCell::new(true),
            }),
            IrOpcode::hq_cast(HqCastFields(IrType::Int)),
            block(),
        ]
    });

    let last_step = result_step(if other_argument {
        vec![IrOpcode::data_lengthoflist(DataLengthoflistFields {
            list: list.clone(),
        })]
        .into_iter()
        .chain(if add_one_to_length {
            vec![
                IrOpcode::hq_integer(HqIntegerFields(1)),
                IrOpcode::operator_add,
            ]
        } else {
            vec![]
        })
        .chain(vec![
            IrOpcode::data_variable(DataVariableFields {
                var: RefCell::new(extra_var.clone()),
                local_read: RefCell::new(true),
            }),
            block(),
        ])
        .collect()
    } else {
        vec![IrOpcode::data_lengthoflist(DataLengthoflistFields {
            list: list.clone(),
        })]
        .into_iter()
        .chain(if add_one_to_length {
            vec![
                IrOpcode::hq_integer(HqIntegerFields(1)),
                IrOpcode::operator_add,
            ]
        } else {
            vec![]
        })
        .chain(vec![block()])
        .collect()
    });

    let random_step = result_step(if other_argument {
        vec![
            IrOpcode::hq_integer(HqIntegerFields(1)),
            IrOpcode::data_lengthoflist(DataLengthoflistFields { list: list.clone() }),
        ]
        .into_iter()
        .chain(if add_one_to_length {
            vec![
                IrOpcode::hq_integer(HqIntegerFields(1)),
                IrOpcode::operator_add,
            ]
        } else {
            vec![]
        })
        .chain(vec![
            IrOpcode::operator_random,
            IrOpcode::data_variable(DataVariableFields {
                var: RefCell::new(extra_var.clone()),
                local_read: RefCell::new(true),
            }),
            block(),
        ])
        .collect()
    } else {
        vec![
            IrOpcode::hq_integer(HqIntegerFields(1)),
            IrOpcode::data_lengthoflist(DataLengthoflistFields { list: list.clone() }),
        ]
        .into_iter()
        .chain(if add_one_to_length {
            vec![
                IrOpcode::hq_integer(HqIntegerFields(1)),
                IrOpcode::operator_add,
            ]
        } else {
            vec![]
        })
        .chain(vec![IrOpcode::operator_random, block()])
        .collect()
    });

    let default_step = if let Some(default_block) = default_output {
        result_step(vec![default_block.clone()])
    } else {
        Rc::new(RefCell::new(Step::new(
            None,
            context.clone(),
            vec![],
            Weak::clone(project),
            false,
        )))
    };

    let not_any_step = maybe_all_block.map(|all_block| {
        let all_step = Rc::new(RefCell::new(Step::new(
            None,
            context.clone(),
            vec![all_block],
            Weak::clone(project),
            false,
        )));

        Rc::new(RefCell::new(Step::new(
            None,
            context.clone(),
            vec![
                IrOpcode::data_variable(DataVariableFields {
                    var: RefCell::new(text_var.clone()),
                    local_read: RefCell::new(true),
                }),
                IrOpcode::hq_text(HqTextFields("all".into())),
                IrOpcode::operator_equals,
                IrOpcode::control_if_else(ControlIfElseFields {
                    branch_if: all_step,
                    branch_else: Rc::clone(&default_step),
                }),
            ],
            Weak::clone(project),
            false,
        )))
    });

    let not_random_step = Rc::new(RefCell::new(Step::new(
        None,
        context.clone(),
        vec![
            IrOpcode::data_variable(DataVariableFields {
                var: RefCell::new(text_var.clone()),
                local_read: RefCell::new(true),
            }),
            IrOpcode::hq_text(HqTextFields("any".into())),
            IrOpcode::operator_equals,
            IrOpcode::control_if_else(ControlIfElseFields {
                branch_if: Rc::clone(&random_step),
                branch_else: not_any_step.unwrap_or(default_step),
            }),
        ],
        Weak::clone(project),
        false,
    )));

    let not_last_step = Rc::new(RefCell::new(Step::new(
        None,
        context.clone(),
        vec![
            IrOpcode::data_variable(DataVariableFields {
                var: RefCell::new(text_var.clone()),
                local_read: RefCell::new(true),
            }),
            IrOpcode::hq_text(HqTextFields("random".into())),
            IrOpcode::operator_equals,
            IrOpcode::control_if_else(ControlIfElseFields {
                branch_if: random_step,
                branch_else: not_random_step,
            }),
        ],
        Weak::clone(project),
        false,
    )));

    let not_int_step = Rc::new(RefCell::new(Step::new(
        None,
        context.clone(),
        vec![
            IrOpcode::data_variable(DataVariableFields {
                var: RefCell::new(text_var.clone()),
                local_read: RefCell::new(true),
            }),
            IrOpcode::hq_text(HqTextFields("last".into())),
            IrOpcode::operator_equals,
            IrOpcode::control_if_else(ControlIfElseFields {
                branch_if: last_step,
                branch_else: not_last_step,
            }),
        ],
        Weak::clone(project),
        false,
    )));

    // we do some silly shennanigans with swapping to make sure that the SSA optimiser stays happy
    Ok(if other_argument {
        vec![IrOpcode::hq_swap]
    } else {
        vec![]
    }
    .into_iter()
    .chain(vec![
        IrOpcode::hq_dup,
        IrOpcode::hq_cast(HqCastFields(IrType::String)),
        IrOpcode::hq_swap,
        IrOpcode::hq_cast(HqCastFields(IrType::Int)),
        IrOpcode::data_setvariableto(DataSetvariabletoFields {
            var: RefCell::new(int_var.clone()),
            local_write: RefCell::new(true),
            first_write: RefCell::new(true),
        }),
        IrOpcode::data_setvariableto(DataSetvariabletoFields {
            var: RefCell::new(text_var),
            local_write: RefCell::new(true),
            first_write: RefCell::new(true),
        }),
    ])
    .chain(if other_argument {
        vec![IrOpcode::data_setvariableto(DataSetvariabletoFields {
            var: RefCell::new(extra_var),
            local_write: RefCell::new(true),
            first_write: RefCell::new(true),
        })]
    } else {
        vec![]
    })
    .chain(vec![
        IrOpcode::data_variable(DataVariableFields {
            var: RefCell::new(int_var),
            local_read: RefCell::new(true),
        }),
        IrOpcode::hq_integer(HqIntegerFields(0)),
        IrOpcode::operator_gt,
        IrOpcode::control_if_else(ControlIfElseFields {
            branch_if: int_step,
            branch_else: not_int_step,
        }),
    ])
    .chain(if has_output {
        vec![IrOpcode::data_variable(DataVariableFields {
            var: RefCell::new(result_var),
            local_read: RefCell::new(true),
        })]
    } else {
        vec![]
    })
    .collect())
}
