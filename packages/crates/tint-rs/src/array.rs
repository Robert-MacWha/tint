/// Creates an array of size N by trying to call the provided fn for each index.
pub fn try_from_fn<T, E, const N: usize>(
    mut f: impl FnMut(usize) -> Result<T, E>,
) -> Result<[T; N], E> {
    Ok((0..N)
        .map(&mut f)
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .unwrap_or_else(|_| unreachable!()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_from_fn_ok() {
        let arr: Result<[i32; 5], _> = try_from_fn(|i| Ok::<i32, ()>(i as i32));
        assert_eq!(arr.unwrap(), [0, 1, 2, 3, 4]);
    }

    #[test]
    fn try_from_fn_err() {
        let arr: Result<[i32; 5], _> = try_from_fn(|i| {
            if i == 3 {
                Err(())
            } else {
                Ok::<i32, ()>(i as i32)
            }
        });
        assert!(arr.is_err());
    }
}
