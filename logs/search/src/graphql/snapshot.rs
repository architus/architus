use std::convert::TryInto;
use std::fmt;
use std::str::FromStr;

/// Represents an opaque snapshot token that just includes an inner timestamp.
/// Used for stateless pagination sessions where we want some stability in the results,
/// so it takes advantage of the `ingestion_timestamp` field attached to each document
/// by only including entries with an timestamp less than or equal to the token (if given)
#[derive(Debug, Clone)]
pub struct Token(pub u64);

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &base64::encode(self.0.to_be_bytes()))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParsingError {
    #[error("could not decode snapshot token base64: {0}")]
    FailedBase64Decode(base64::DecodeError),
    #[error("could not decode inner bytes to u64: bad length (expected: 8, actual: {0})")]
    FailedU64Decode(usize),
}

#[juniper::graphql_scalar(description = "SnapshotToken")]
impl<S> juniper::GraphQLScalar for Token
where
    S: juniper::ScalarValue,
{
    fn resolve(&self) -> juniper::Value {
        juniper::Value::scalar(self.to_string())
    }

    fn from_input_value(v: &juniper::InputValue) -> Option<Self> {
        v.as_scalar_value()
            .and_then(juniper::ScalarValue::as_str)
            .and_then(|s| FromStr::from_str(s).ok())
    }

    fn from_str(value: juniper::parser::ScalarToken) -> juniper::ParseScalarResult<S> {
        <String as juniper::ParseScalarValue<S>>::from_str(value)
    }
}

impl FromStr for Token {
    type Err = ParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let base64_bytes = base64::decode(s).map_err(ParsingError::FailedBase64Decode)?;
        let u64_bytes = base64_bytes
            .try_into()
            .map_err(|original: Vec<u8>| ParsingError::FailedU64Decode(original.len()))?;
        Ok(Self(u64::from_be_bytes(u64_bytes)))
    }
}
