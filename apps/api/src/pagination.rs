use serde::Deserialize;

pub const PAGE_SIZE: i64 = 50;
const MAX_OFFSET: i64 = 1_000_000;

#[derive(Clone, Copy, Debug)]
pub struct PageWindow {
    pub limit: i64,
    pub offset: i64,
}

impl PageWindow {
    pub fn new(offset: Option<i64>, limit: Option<i64>) -> Option<Self> {
        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(PAGE_SIZE);
        if !(0..=MAX_OFFSET).contains(&offset) || !(1..=100).contains(&limit) {
            return None;
        }
        Some(Self { limit, offset })
    }
}

#[derive(Clone, Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PageQuery {
    /// Maximum number of items to return (defaults to 50, maximum 100).
    #[param(minimum = 1, maximum = 100)]
    pub limit: Option<i64>,
    /// Number of items to skip.
    #[param(minimum = 0, maximum = 1000000)]
    pub offset: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TaskBoardPageQuery {
    /// Maximum number of Tasks to return (defaults to 50, maximum 100).
    #[param(minimum = 1, maximum = 100)]
    pub tasks_limit: Option<i64>,
    /// Number of Tasks to skip. Pages contain at most 50 items.
    #[param(minimum = 0, maximum = 1000000)]
    pub tasks_offset: Option<i64>,
    /// Maximum number of To-do lists to return (defaults to 50, maximum 100).
    #[param(minimum = 1, maximum = 100)]
    pub lists_limit: Option<i64>,
    /// Number of To-do lists to skip. Pages contain at most 50 items.
    #[param(minimum = 0, maximum = 1000000)]
    pub lists_offset: Option<i64>,
}

pub struct Page<T> {
    pub items: Vec<T>,
    pub next_offset: Option<i64>,
}

impl<T> Page<T> {
    pub fn from_rows(mut rows: Vec<T>, window: PageWindow) -> Self {
        let has_more = rows.len() as i64 > window.limit;
        rows.truncate(window.limit as usize);
        Self {
            items: rows,
            next_offset: has_more.then_some(window.offset + window.limit),
        }
    }
}
