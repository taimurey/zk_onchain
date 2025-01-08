// Helper function for array conversion
pub fn vec_to_array<T, const N: usize>(vec: Vec<T>, field_name: &str) -> anyhow::Result<[T; N]>
where
    T: Default + Copy,
{
    if vec.len() != N {
        anyhow::bail!(
            "Invalid length for {} field: expected {}, got {}",
            field_name,
            N,
            vec.len()
        );
    }
    let mut arr = [T::default(); N];
    arr.copy_from_slice(&vec);
    Ok(arr)
}
