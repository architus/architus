use crate::gateway::ProcessorContext;
use anyhow::Context as _;
use jmespath::{Expression, Rcvar, Variable};

/// Wraps a `JMESPath` `Expression` and adds static compilation + extraction utilities on top of it
#[derive(Clone, Debug)]
pub struct Path(Expression<'static>);

#[allow(clippy::fallible_impl_from)]
impl From<&'static str> for Path {
    fn from(s: &'static str) -> Self {
        let compiled = jmespath::compile(s);
        assert!(compiled.is_ok(), "Compilation of JMESPath '{}' failed", s);
        Self(compiled.unwrap())
    }
}

impl Path {
    /// Attempts to search for a value in the JSON value, returning a single value or an error
    pub fn search(&self, value: &serde_json::Value) -> Result<Rcvar, anyhow::Error> {
        self.0
            .search(value)
            .with_context(|| format!("searching failed for expression '{}'", self.0))
    }

    // Attempts to search + parse a value from the JSON value,
    // returning some type T if successful
    pub fn extract<'a, 'b, T>(
        &self,
        value: &'a serde_json::Value,
        extractor: &'a dyn Fn(&Variable, &ProcessorContext<'_>) -> Result<T, anyhow::Error>,
        ctx: &ProcessorContext<'b>,
    ) -> Result<T, anyhow::Error> {
        self.search(value).and_then(|rc_var| {
            (extractor)(rc_var.as_ref(), ctx).with_context(|| {
                format!(
                    "failed to convert value '{:?}' into type '{}'",
                    rc_var,
                    std::any::type_name::<T>()
                )
            })
        })
    }
}

/// Small utility macro that uses the `lazy_static` crate
/// to create a static `Path` instance that is compiled when it is first used,
/// and then shared after that.
/// This panics (upon first use) if compilation of the path fails.
macro_rules! json_path {
    ($a:expr) => {{
        ::lazy_static::lazy_static! {
            static ref PATH_STATIC: Path = Path::from($a);
        }

        &PATH_STATIC
    }};
}

// Thanks clippy :)
#[allow(
    clippy::useless_attribute,
    clippy::redundant_pub_crate,
    clippy::single_component_path_imports
)]
pub(crate) use json_path;
