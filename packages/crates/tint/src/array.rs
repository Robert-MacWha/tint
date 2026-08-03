/// Creates an array of size N by trying to call the provided fn for each index.
///
/// # Errors
/// Errors if the provided fn returns an error for any index.
#[allow(clippy::unreachable)]
pub fn try_from_fn<T, E, const N: usize>(
    mut f: impl FnMut(usize) -> Result<T, E>,
) -> Result<[T; N], E> {
    // SAFETY: The `collect` call below will produce a `Vec<T>` of length N, which
    // `try_into` will always succeed in converting to `[T; N]`.
    Ok((0..N)
        .map(&mut f)
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .unwrap_or_else(|_| unreachable!()))
}

/// Creates an array of size N by trying to map the provided fn over each element of the input array.
///
/// # Errors
/// Errors if the provided fn returns an error for any element of the input array.
#[allow(clippy::unreachable)]
pub fn try_map<T, U, E, const N: usize>(
    arr: [T; N],
    mut f: impl FnMut(T) -> Result<U, E>,
) -> Result<[U; N], E> {
    // SAFETY: The `collect` call below will produce a `Vec<U>` of length N, which
    // `try_into` will always succeed in converting to `[U; N]`.
    Ok(arr
        .into_iter()
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
