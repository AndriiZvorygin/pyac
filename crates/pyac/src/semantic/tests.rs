//! The semantic decoders, checked against golden codelets.

use super::super::semantic::*;
use super::words::*;
use crate::WORDS_PER_CODELET;

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_PLUS: [u16; WORDS_PER_CODELET] = GOLDEN_NUMERIC_PLUS_CODELET;

    #[test]
    fn decodes_and_executes_the_shared_fixture() {
        let instruction = decode_numeric_plus(&ONE_PLUS).expect("golden codelet is valid");
        assert_eq!(instruction.ob, 1);
        assert_eq!(instruction.to, 1);
        assert_eq!(execute_numeric_plus_codelet(&ONE_PLUS), Ok(2));
    }

    #[test]
    fn keeps_saturation_and_rejects_noisy_tail() {
        let mut saturated = ONE_PLUS;
        saturated[2] = u16::MAX;
        saturated[5] = 1;
        assert_eq!(execute_numeric_plus_codelet(&saturated), Ok(u16::MAX));

        let mut noisy = ONE_PLUS;
        noisy[9] = 1;
        let error = decode_numeric_plus(&noisy).expect_err("tail must be zero");
        assert_eq!(error.code(), "nonzero-tail");
    }

    fn arithmetic_codelet(operator: NumericOperator, ob: u16, to: u16) -> [u16; WORDS_PER_CODELET] {
        [
            HEAD,
            NUMBER_QUOTE,
            ob,
            ACCUSATIVE_CASE,
            NUMBER_QUOTE,
            to,
            DESTINATION_CASE,
            operator.word(),
            DEONTIC_MOOD,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ]
    }

    #[test]
    fn executes_the_other_proven_numeric_operators() {
        assert_eq!(
            execute_numeric_arithmetic_codelet(&arithmetic_codelet(
                NumericOperator::Subtract,
                2,
                9
            )),
            Ok(0)
        );
        assert_eq!(
            execute_numeric_arithmetic_codelet(&arithmetic_codelet(
                NumericOperator::Multiply,
                u16::MAX,
                u16::MAX
            )),
            Ok(4_294_836_225)
        );
        assert_eq!(
            execute_numeric_arithmetic_codelet(&arithmetic_codelet(NumericOperator::Divide, 5, 2)),
            Ok(2)
        );
        assert_eq!(
            execute_numeric_arithmetic_codelet(&arithmetic_codelet(NumericOperator::Xor, 7, 3)),
            Ok(4)
        );
    }

    #[test]
    fn executes_exponential_and_remainder_like_the_reference_backend() {
        assert_eq!(
            execute_numeric_arithmetic_codelet(&arithmetic_codelet(
                NumericOperator::Exponential,
                2,
                10
            )),
            Ok(1024)
        );
        assert_eq!(
            execute_numeric_arithmetic_codelet(&arithmetic_codelet(
                NumericOperator::Exponential,
                2,
                20
            )),
            Ok(u64::from(u16::MAX))
        );
        assert_eq!(
            execute_numeric_arithmetic_codelet(&arithmetic_codelet(NumericOperator::Remains, 9, 4)),
            Ok(1)
        );
    }

    fn invert_codelet(ob: u16, bound: u16) -> [u16; WORDS_PER_CODELET] {
        [
            HEAD,
            NUMBER_QUOTE,
            ob,
            ACCUSATIVE_CASE,
            NUMBER_QUOTE,
            bound,
            DATIVE_CASE,
            INVERT,
            DEONTIC_MOOD,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ]
    }

    #[test]
    fn executes_the_legacy_invert_shape() {
        assert_eq!(
            execute_numeric_invert_codelet(&invert_codelet(5, 0)),
            Ok(-5)
        );
        assert_eq!(
            execute_numeric_invert_codelet(&invert_codelet(5, 9)).expect_err("only a zero bound"),
            SemanticError::UnsupportedBound { actual: 9 }
        );
    }

    #[test]
    fn reports_arithmetic_operator_errors() {
        let mut unknown = arithmetic_codelet(NumericOperator::Plus, 1, 1);
        unknown[7] = 0x1234;
        assert_eq!(
            decode_numeric_arithmetic(&unknown).expect_err("unknown operator"),
            SemanticError::UnsupportedOperator { actual: 0x1234 }
        );

        let divide_by_zero = arithmetic_codelet(NumericOperator::Divide, 8, 0);
        assert_eq!(
            execute_numeric_arithmetic_codelet(&divide_by_zero).expect_err("division by zero"),
            SemanticError::DivisionByZero
        );

        let remainder_by_zero = arithmetic_codelet(NumericOperator::Remains, 9, 0);
        assert_eq!(
            execute_numeric_arithmetic_codelet(&remainder_by_zero).expect_err("division by zero"),
            SemanticError::DivisionByZero
        );
    }
}
