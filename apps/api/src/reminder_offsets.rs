pub(crate) const PRESETS: [i32; 7] = [15, 30, 60, 120, 1440, 2880, 10080];

pub(crate) fn validated(offsets: &[i32]) -> Option<Vec<i32>> {
    if offsets.len() > 5 || offsets.iter().any(|offset| !PRESETS.contains(offset)) {
        return None;
    }
    let mut sorted = offsets.to_vec();
    sorted.sort_unstable();
    (!sorted.windows(2).any(|pair| pair[0] == pair[1])).then_some(sorted)
}
