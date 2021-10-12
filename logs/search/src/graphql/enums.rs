use crate::rpc::logs::event::{AgentSpecialType, EntityType, EventOrigin, EventType};

/// Represents a trait over the numeric cast for field-less enum discriminants
pub trait FilterableEnum {
    fn discriminant(&self) -> i32;
}

impl FilterableEnum for EventType {
    fn discriminant(&self) -> i32 {
        *self as i32
    }
}

impl FilterableEnum for EventOrigin {
    fn discriminant(&self) -> i32 {
        *self as i32
    }
}

impl FilterableEnum for EntityType {
    fn discriminant(&self) -> i32 {
        *self as i32
    }
}

impl FilterableEnum for AgentSpecialType {
    fn discriminant(&self) -> i32 {
        *self as i32
    }
}
