/// 分页信息
#[derive(Debug, Clone)]
pub struct PageInfo {
    pub current_page: u64,
    pub total_pages: u64,
    pub total_count: u64,
    pub has_previous: bool,
    pub has_next: bool,
}

impl PageInfo {
    /// 根据总数和每页大小计算分页信息
    pub fn new(total_count: u64, page: u64, page_size: u64) -> Self {
        let total_pages = if total_count == 0 {
            1
        } else {
            (total_count + page_size - 1) / page_size
        };
        let current_page = page.min(total_pages).max(1);
        Self {
            current_page,
            total_pages,
            total_count,
            has_previous: current_page > 1,
            has_next: current_page < total_pages,
        }
    }
}
