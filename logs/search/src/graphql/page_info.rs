use std::convert::TryFrom;

/// Includes metadata about a search's pagination
#[derive(juniper::GraphQLObject)]
pub struct PageInfo {
    current_page: i32,
    has_previous_page: bool,
    has_next_page: bool,
    item_count: i32,
    page_count: i32,
    per_page: i32,
    total_count: i32,
}

pub struct PageInfoInputs {
    pub count: usize,
    pub total: usize,
    pub after: usize,
    pub limit: usize,
    pub max_pagination: usize,
}

impl PageInfo {
    pub fn new(inputs: PageInfoInputs) -> Self {
        let current_page = inputs.after / inputs.limit;
        let previous_page_start = inputs.after.saturating_sub(inputs.limit);
        let next_page_start = inputs.after.saturating_add(inputs.limit);
        let next_page_end = next_page_start.saturating_add(inputs.limit);
        let total_count = usize::try_from(inputs.total).unwrap_or(0);
        let total_pageable = total_count - (inputs.after % inputs.limit);
        let page_count = (total_pageable.saturating_sub(1) / inputs.limit).saturating_add(1);

        Self {
            current_page: i32::try_from(current_page).unwrap_or(i32::max_value()),
            has_previous_page: current_page > 0 && previous_page_start <= total_count,
            has_next_page: next_page_end <= inputs.max_pagination && next_page_start < total_count,
            item_count: i32::try_from(inputs.count).unwrap_or(i32::max_value()),
            page_count: i32::try_from(page_count).unwrap_or(i32::max_value()),
            per_page: i32::try_from(inputs.limit).unwrap_or(i32::max_value()),
            total_count: i32::try_from(total_count).unwrap_or(i32::max_value()),
        }
    }
}
