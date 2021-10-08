use crate::elasticsearch::filters::{
    EmptyFilter, ExactSetFilter, IdFilter, IdSetFilter, RangeFilter, SetFilter, TermFilter,
    TextFilter, WildcardFilter,
};
use crate::elasticsearch::SearchParams;
use crate::graphql::enums::FilterableEnum;
use juniper::{graphql_value, FieldError, FieldResult};
use std::str::FromStr;

pub trait QueryInput {
    fn to_elasticsearch(&self, params: &mut SearchParams) -> FieldResult<()>;
}

impl<T: QueryInput> QueryInput for Option<T> {
    fn to_elasticsearch(&self, params: &mut SearchParams) -> FieldResult<()> {
        if let Some(inner_value) = self {
            inner_value.to_elasticsearch(params)?;
        }

        Ok(())
    }
}

/// Parses a uint string into the actual u64 value, or returns a `FieldError`
fn parse_uint_string(s: impl AsRef<str>) -> FieldResult<u64> {
    let s = s.as_ref();
    s.parse::<u64>().map_err(|err| {
        let message = format!("Could not parse unsigned int from '{}'", s);
        let internal = format!("{:?}", err);
        FieldError::new(
            message,
            graphql_value!({
                "internal": internal,
            }),
        )
    })
}

/// Parses a slice of uint strings into the actual u64 values, or returns a `FieldError`
fn parse_uint_string_slice(slice: &[impl AsRef<str>]) -> FieldResult<Vec<u64>> {
    slice
        .iter()
        .map(parse_uint_string)
        .collect::<FieldResult<Vec<_>>>()
}

/// Filter input object that allows for the filtering of an unsigned integer value
/// (represented in the schema as a String)
#[derive(juniper::GraphQLInputObject)]
pub struct UIntStringFilterInput {
    eq: Option<String>,
    ne: Option<String>,
    gt: Option<String>,
    gte: Option<String>,
    lt: Option<String>,
    lte: Option<String>,
    #[graphql(name = "in")]
    r#in: Option<Vec<String>>,
    nin: Option<Vec<String>>,
}

impl UIntStringFilterInput {
    pub fn apply(&self, params: &mut SearchParams, field_name: impl AsRef<str>) -> FieldResult<()> {
        let field_name = field_name.as_ref();

        TermFilter::EqualTo.apply(
            self.eq.as_ref().map(parse_uint_string).transpose()?,
            params,
            &field_name,
        )?;
        TermFilter::NotEqualTo.apply(
            self.ne.as_ref().map(parse_uint_string).transpose()?,
            params,
            &field_name,
        )?;
        RangeFilter::GreaterThan.apply(
            self.gt.as_ref().map(parse_uint_string).transpose()?,
            params,
            &field_name,
        )?;
        RangeFilter::GreaterThanOrEqualTo.apply(
            self.gte.as_ref().map(parse_uint_string).transpose()?,
            params,
            &field_name,
        )?;
        RangeFilter::LessThan.apply(
            self.lt.as_ref().map(parse_uint_string).transpose()?,
            params,
            &field_name,
        )?;
        RangeFilter::LessThanOrEqualTo.apply(
            self.lte.as_ref().map(parse_uint_string).transpose()?,
            params,
            &field_name,
        )?;
        SetFilter::In.apply(
            self.r#in
                .as_ref()
                .map(|v| parse_uint_string_slice(&v))
                .transpose()?,
            params,
            &field_name,
        )?;
        SetFilter::NotIn.apply(
            self.nin
                .as_ref()
                .map(|v| parse_uint_string_slice(&v))
                .transpose()?,
            params,
            &field_name,
        )?;

        Ok(())
    }
}

/// Filter input object that allows for the filtering of a document ID
/// (represented in the schema as a String)
#[derive(juniper::GraphQLInputObject)]
pub struct IdFilterInput {
    eq: Option<String>,
    ne: Option<String>,
    #[graphql(name = "in")]
    r#in: Option<Vec<String>>,
    nin: Option<Vec<String>>,
}

impl QueryInput for IdFilterInput {
    fn to_elasticsearch(&self, params: &mut SearchParams) -> FieldResult<()> {
        // Ensure each filter doesn't contain empty values
        if self.eq.as_ref().map(String::is_empty).unwrap_or(false) {
            return Err(FieldError::new(
                "id.eq filter cannot contain empty values",
                graphql_value!({"source": "IdFilterInput"}),
            ));
        }
        if self.ne.as_ref().map(String::is_empty).unwrap_or(false) {
            return Err(FieldError::new(
                "id.ne filter cannot contain empty values",
                graphql_value!({"source": "IdFilterInput"}),
            ));
        }
        if self
            .r#in
            .as_ref()
            .map(|s| s.iter().any(String::is_empty))
            .unwrap_or(false)
        {
            return Err(FieldError::new(
                "id.in filter cannot contain empty values",
                graphql_value!({"source": "IdFilterInput"}),
            ));
        }
        if self
            .nin
            .as_ref()
            .map(|s| s.iter().any(String::is_empty))
            .unwrap_or(false)
        {
            return Err(FieldError::new(
                "id.nin filter cannot contain empty values",
                graphql_value!({"source": "IdFilterInput"}),
            ));
        }

        IdFilter::EqualTo.apply(self.eq.clone(), params);
        IdFilter::NotEqualTo.apply(self.ne.clone(), params);
        IdSetFilter::In.apply(self.r#in.clone(), params);
        IdSetFilter::NotIn.apply(self.nin.clone(), params);

        Ok(())
    }
}

/// Filter input object that allows for the filtering of an enum
/// (represented in the schema as a String)
#[derive(juniper::GraphQLInputObject)]
pub struct EnumFilterInput {
    eq: Option<String>,
    ne: Option<String>,
    #[graphql(name = "in")]
    r#in: Option<Vec<String>>,
    nin: Option<Vec<String>>,
}

impl EnumFilterInput {
    fn try_convert<A, T>(string: A) -> FieldResult<i32>
    where
        A: AsRef<str>,
        T: FilterableEnum + FromStr,
        <T as FromStr>::Err: std::fmt::Debug,
    {
        let string = string.as_ref();
        let value: Result<T, _> = FromStr::from_str(string);
        match value {
            Ok(value) => Ok(value.discriminant()),
            Err(err) => {
                let message = format!(
                    "Could not convert enum value '{}' to desired type '{}'",
                    string,
                    std::any::type_name::<T>()
                );
                let internal = format!("{:?}", err);
                Err(FieldError::new(
                    message,
                    graphql_value!({
                        "internal": internal,
                    }),
                ))
            }
        }
    }

    pub fn apply<T>(&self, params: &mut SearchParams, field_name: &str) -> FieldResult<()>
    where
        T: FilterableEnum + FromStr,
        <T as FromStr>::Err: std::fmt::Debug,
    {
        TermFilter::EqualTo.apply(
            self.eq
                .as_ref()
                .map(Self::try_convert::<_, T>)
                .transpose()?,
            params,
            &field_name,
        )?;
        TermFilter::NotEqualTo.apply(
            self.ne
                .as_ref()
                .map(Self::try_convert::<_, T>)
                .transpose()?,
            params,
            &field_name,
        )?;
        SetFilter::In.apply(
            self.r#in
                .as_ref()
                .map(|v| {
                    v.iter()
                        .map(Self::try_convert::<_, T>)
                        .collect::<FieldResult<Vec<_>>>()
                })
                .transpose()?,
            params,
            &field_name,
        )?;
        SetFilter::NotIn.apply(
            self.nin
                .as_ref()
                .map(|v| {
                    v.iter()
                        .map(Self::try_convert::<_, T>)
                        .collect::<FieldResult<Vec<_>>>()
                })
                .transpose()?,
            params,
            &field_name,
        )?;

        Ok(())
    }
}

/// Filter input object that allows for the filtering of an optional unsigned integer value
/// (represented in the schema as a String)
#[derive(juniper::GraphQLInputObject)]
pub struct UIntStringOptionFilterInput {
    eq: Option<String>,
    ne: Option<String>,
    gt: Option<String>,
    gte: Option<String>,
    lt: Option<String>,
    lte: Option<String>,
    #[graphql(name = "in")]
    r#in: Option<Vec<String>>,
    nin: Option<Vec<String>>,
    present: Option<bool>,
    absent: Option<bool>,
}

impl UIntStringOptionFilterInput {
    pub fn apply(&self, params: &mut SearchParams, field_name: impl AsRef<str>) -> FieldResult<()> {
        let field_name = field_name.as_ref();

        TermFilter::EqualTo.apply(
            self.eq.as_ref().map(parse_uint_string).transpose()?,
            params,
            &field_name,
        )?;
        TermFilter::NotEqualTo.apply(
            self.ne.as_ref().map(parse_uint_string).transpose()?,
            params,
            &field_name,
        )?;
        RangeFilter::GreaterThan.apply(
            self.gt.as_ref().map(parse_uint_string).transpose()?,
            params,
            &field_name,
        )?;
        RangeFilter::GreaterThanOrEqualTo.apply(
            self.gte.as_ref().map(parse_uint_string).transpose()?,
            params,
            &field_name,
        )?;
        RangeFilter::LessThan.apply(
            self.lt.as_ref().map(parse_uint_string).transpose()?,
            params,
            &field_name,
        )?;
        RangeFilter::LessThanOrEqualTo.apply(
            self.lte.as_ref().map(parse_uint_string).transpose()?,
            params,
            &field_name,
        )?;
        SetFilter::In.apply(
            self.r#in
                .as_ref()
                .map(|v| parse_uint_string_slice(&v))
                .transpose()?,
            params,
            &field_name,
        )?;
        SetFilter::NotIn.apply(
            self.nin
                .as_ref()
                .map(|v| parse_uint_string_slice(&v))
                .transpose()?,
            params,
            &field_name,
        )?;
        TermFilter::EqualTo.apply(
            self.present.filter(|b| *b).map(|_| 0_u64),
            params,
            &field_name,
        )?;
        TermFilter::NotEqualTo.apply(
            self.absent.filter(|b| *b).map(|_| 0_u64),
            params,
            &field_name,
        )?;

        Ok(())
    }
}

/// Filter input object that allows for the filtering of a text value
/// (a string that is analyzed in Elasticsearch using full-text analysis)
#[derive(juniper::GraphQLInputObject)]
pub struct TextFilterInput {
    empty: Option<bool>,
    non_empty: Option<bool>,
    query: Option<String>,
    r#match: Option<String>,
}

impl TextFilterInput {
    pub fn apply(&self, params: &mut SearchParams, field_name: impl AsRef<str>) -> FieldResult<()> {
        let field_name = field_name.as_ref();

        TermFilter::EqualTo.apply(
            self.empty.filter(|b| *b).map(|_| String::from("")),
            params,
            &field_name,
        )?;
        TermFilter::NotEqualTo.apply(
            self.non_empty.filter(|b| *b).map(|_| String::from("")),
            params,
            &field_name,
        )?;
        TextFilter::Query.apply(self.query.clone(), params, &field_name);
        TextFilter::Match.apply(self.r#match.clone(), params, &field_name);

        Ok(())
    }
}

/// Filter input object that allows for the filtering of a string value
#[derive(juniper::GraphQLInputObject)]
pub struct StringFilterInput {
    eq: Option<String>,
    ne: Option<String>,
    r#in: Option<Vec<String>>,
    nin: Option<Vec<String>>,
    empty: Option<bool>,
    non_empty: Option<bool>,
    wildcard: Option<String>,
}

impl StringFilterInput {
    pub fn apply(&self, params: &mut SearchParams, field_name: impl AsRef<str>) -> FieldResult<()> {
        let field_name = field_name.as_ref();

        TermFilter::EqualTo.apply(self.eq.clone(), params, &field_name)?;
        TermFilter::NotEqualTo.apply(self.ne.clone(), params, &field_name)?;
        SetFilter::In.apply(self.r#in.clone(), params, &field_name)?;
        SetFilter::NotIn.apply(self.nin.clone(), params, &field_name)?;
        TermFilter::EqualTo.apply(
            self.empty.filter(|b| *b).map(|_| String::from("")),
            params,
            &field_name,
        )?;
        TermFilter::NotEqualTo.apply(
            self.non_empty.filter(|b| *b).map(|_| String::from("")),
            params,
            &field_name,
        )?;
        WildcardFilter.apply(self.wildcard.clone(), params, &field_name);

        Ok(())
    }
}

/// Filter input object that allows for the filtering of a set of unsigned integer values
/// (represented in the schema as Strings)
#[derive(juniper::GraphQLInputObject)]
pub struct UIntStringSetFilterInput {
    empty: Option<bool>,
    non_empty: Option<bool>,
    contains: Option<String>,
    does_not_contain: Option<String>,
    contains_any: Option<Vec<String>>,
    contains_none: Option<Vec<String>>,
    contains_all: Option<Vec<String>>,
}

impl UIntStringSetFilterInput {
    pub fn apply(&self, params: &mut SearchParams, field_name: impl AsRef<str>) -> FieldResult<()> {
        let field_name = field_name.as_ref();

        EmptyFilter::Empty.apply(self.empty.unwrap_or(false), params, &field_name);
        EmptyFilter::NotEmpty.apply(self.non_empty.unwrap_or(false), params, &field_name);
        TermFilter::EqualTo.apply(
            self.contains.as_ref().map(parse_uint_string).transpose()?,
            params,
            &field_name,
        )?;
        TermFilter::NotEqualTo.apply(
            self.does_not_contain
                .as_ref()
                .map(parse_uint_string)
                .transpose()?,
            params,
            &field_name,
        )?;
        SetFilter::In.apply(
            self.contains_any
                .as_ref()
                .map(|v| parse_uint_string_slice(&v))
                .transpose()?,
            params,
            &field_name,
        )?;
        SetFilter::NotIn.apply(
            self.contains_none
                .as_ref()
                .map(|v| parse_uint_string_slice(&v))
                .transpose()?,
            params,
            &field_name,
        )?;
        ExactSetFilter::ContainsAll.apply(
            self.contains_none
                .as_ref()
                .map(|v| parse_uint_string_slice(&v))
                .transpose()?,
            params,
            &field_name,
        )?;

        Ok(())
    }
}

/// Filter input object that allows for the filtering of a set of strings
#[derive(juniper::GraphQLInputObject)]
pub struct StringSetFilterInput {
    empty: Option<bool>,
    non_empty: Option<bool>,
    contains: Option<String>,
    does_not_contain: Option<String>,
    contains_any: Option<Vec<String>>,
    contains_none: Option<Vec<String>>,
    contains_all: Option<Vec<String>>,
    contains_wildcard: Option<String>,
}

impl StringSetFilterInput {
    pub fn apply(&self, params: &mut SearchParams, field_name: impl AsRef<str>) -> FieldResult<()> {
        let field_name = field_name.as_ref();

        EmptyFilter::Empty.apply(self.empty.unwrap_or(false), params, &field_name);
        EmptyFilter::NotEmpty.apply(self.non_empty.unwrap_or(false), params, &field_name);
        TermFilter::EqualTo.apply(self.contains.clone(), params, &field_name)?;
        TermFilter::NotEqualTo.apply(self.does_not_contain.clone(), params, &field_name)?;
        SetFilter::In.apply(self.contains_any.clone(), params, &field_name)?;
        SetFilter::NotIn.apply(self.contains_none.clone(), params, &field_name)?;
        ExactSetFilter::ContainsAll.apply(self.contains_none.clone(), params, &field_name)?;
        WildcardFilter.apply(self.contains_wildcard.clone(), params, &field_name);

        Ok(())
    }
}
