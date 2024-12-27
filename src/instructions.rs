pub mod operator;

#[allow(non_camel_case_types)]
pub enum IrOpcode {
    operator_add
}

#[macro_export]
macro_rules! instructions_test {
    ($($type_arg:ident $(,)?)+) => {
        #[cfg(test)]
        pub mod tests {
            use super::{instructions, output_type, IrType};
            #[test]
            fn output_type_fails_when_instructions_fails() {
                $(let $type_arg = IrType::flags().map(|(_, ty)| ty);
                )+
                for ($($type_arg ,)+) in itertools::iproduct!($($type_arg,)+){
                    println!("boop")
                }
            }
        }
    }
}