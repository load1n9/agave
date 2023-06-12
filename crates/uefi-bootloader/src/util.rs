pub(crate) fn calculate_pages(bytes: usize) -> usize {
    ((bytes - 1) / 4096) + 1
}
