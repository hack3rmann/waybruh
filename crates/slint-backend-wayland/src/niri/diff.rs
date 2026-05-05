use std::ops::{Index, IndexMut};

struct Matrix<T> {
    pub n_rows: u32,
    pub n_cols: u32,
    pub values: Vec<T>,
}

impl<T> Matrix<T> {
    pub fn copy_nxm(n_rows: u32, n_cols: u32, value: T) -> Self
    where
        T: Copy,
    {
        Self {
            n_rows,
            n_cols,
            values: vec![value; (n_rows * n_cols) as usize],
        }
    }

    pub fn index(&self, row: usize, col: usize) -> usize {
        col + self.n_cols as usize * row
    }
}

impl<T> Index<(usize, usize)> for Matrix<T> {
    type Output = T;

    fn index(&self, (row, col): (usize, usize)) -> &Self::Output {
        assert!(row < self.n_rows as usize);
        assert!(col < self.n_cols as usize);

        &self.values[self.index(row, col)]
    }
}

impl<T> IndexMut<(usize, usize)> for Matrix<T> {
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut Self::Output {
        assert!(row < self.n_rows as usize);
        assert!(col < self.n_cols as usize);

        let index = self.index(row, col);
        &mut self.values[index]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Edit<T> {
    Equal(T),
    Insert(T),
    Remove(T),
    Replace(T),
}

fn lcs_table<T>(a: &[T], b: &[T], eq: &mut impl FnMut(&T, &T) -> bool) -> Matrix<u16> {
    let mut result = Matrix::<u16>::copy_nxm(a.len() as u32 + 1, b.len() as u32 + 1, 0);

    for i in 0..a.len() {
        for j in 0..b.len() {
            if eq(&a[i], &b[j]) {
                result[(i + 1, j + 1)] = result[(i, j)].checked_add(1).unwrap();
            } else {
                result[(i + 1, j + 1)] = result[(i + 1, j)].max(result[(i, j + 1)]);
            }
        }
    }

    result
}

fn collapse_replaces<T: Clone>(input: &[Edit<T>]) -> Vec<Edit<T>> {
    let mut result = Vec::with_capacity(input.len());
    let mut i = 0;

    while i < input.len() {
        match &input[i] {
            Edit::Equal(x) => {
                result.push(Edit::Equal(x.clone()));
                i += 1;
            }

            Edit::Remove(_) => {
                let start = i;

                // 1. find end of removes
                while i < input.len() && matches!(input[i], Edit::Remove(_)) {
                    i += 1;
                }
                let r_end = i;

                // 2. find end of inserts
                while i < input.len() && matches!(input[i], Edit::Insert(_)) {
                    i += 1;
                }
                let i_end = i;

                let r_len = r_end - start;
                let i_len = i_end - r_end;
                let k = r_len.min(i_len);

                // 3. emit Replace
                for j in 0..k {
                    if let Edit::Insert(val) = &input[r_end + j] {
                        result.push(Edit::Replace(val.clone()));
                    }
                }

                // 4. leftover removes
                for j in k..r_len {
                    if let Edit::Remove(val) = &input[start + j] {
                        result.push(Edit::Remove(val.clone()));
                    }
                }

                // 5. leftover inserts
                for j in k..i_len {
                    if let Edit::Insert(val) = &input[r_end + j] {
                        result.push(Edit::Insert(val.clone()));
                    }
                }
            }

            Edit::Insert(x) => {
                // no preceding remove → cannot merge
                result.push(Edit::Insert(x.clone()));
                i += 1;
            }

            Edit::Replace(_) => {
                result.push(input[i].clone());
                i += 1;
            }
        }
    }

    result
}

pub fn difference<'t, T: Eq>(a: &'t [T], b: &'t [T]) -> Vec<Edit<&'t T>> {
    difference_by(a, b, T::eq)
}

pub fn difference_by<'t, T>(
    a: &'t [T],
    b: &'t [T],
    mut eq: impl FnMut(&T, &T) -> bool,
) -> Vec<Edit<&'t T>> {
    let lcs = lcs_table(a, b, &mut eq);
    let mut result = Vec::new();

    let mut i = a.len();
    let mut j = b.len();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && eq(&a[i - 1], &b[j - 1]) {
            result.push(Edit::Equal(&a[i - 1]));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || lcs[(i, j - 1)] >= lcs[(i - 1, j)]) {
            result.push(Edit::Insert(&b[j - 1]));
            j -= 1;
        } else {
            result.push(Edit::Remove(&a[i - 1]));
            i -= 1;
        }
    }

    result.reverse();
    collapse_replaces(&result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff1() {
        let a = vec![1, 2, 3, 4];
        let b = vec![1, 5, 3, 4];

        let diff = difference(&a, &b);

        assert_eq!(
            diff,
            [
                Edit::Equal(&1),
                Edit::Replace(&5),
                Edit::Equal(&3),
                Edit::Equal(&4)
            ]
        );

        let mut c = a.clone();

        for (i, &diff) in diff.iter().enumerate() {
            match diff {
                Edit::Equal(_) => continue,
                Edit::Insert(&value) => {
                    c.insert(i, value);
                }
                Edit::Remove(_) => {
                    c.remove(i);
                }
                Edit::Replace(&value) => c[i] = value,
            }
        }

        assert_eq!(b, c);
    }

    #[test]
    fn diff2() {
        let a = vec![1, 2, 3, 4, 1, 2, 3];
        let b = vec![1, 2, 3, 1, 2, 3];

        let diff = difference(&a, &b);

        assert_eq!(
            diff,
            [
                Edit::Equal(&1),
                Edit::Equal(&2),
                Edit::Equal(&3),
                Edit::Remove(&4),
                Edit::Equal(&1),
                Edit::Equal(&2),
                Edit::Equal(&3)
            ]
        );

        let mut c = a.clone();

        for (i, &diff) in diff.iter().enumerate() {
            match diff {
                Edit::Equal(_) => continue,
                Edit::Insert(&value) => {
                    c.insert(i, value);
                }
                Edit::Remove(_) => {
                    c.remove(i);
                }
                Edit::Replace(&value) => c[i] = value,
            }
        }

        assert_eq!(b, c);
    }

    #[test]
    fn diff3() {
        let a = vec![1, 2, 3, 4, 1, 2, 3];
        let b = vec![1, 2, 3, 4, 5, 6, 7];

        let diff = difference(&a, &b);

        assert_eq!(
            diff,
            [
                Edit::Equal(&1),
                Edit::Equal(&2),
                Edit::Equal(&3),
                Edit::Equal(&4),
                Edit::Replace(&5),
                Edit::Replace(&6),
                Edit::Replace(&7)
            ]
        );

        let mut c = a.clone();

        for (i, &diff) in diff.iter().enumerate() {
            match diff {
                Edit::Equal(_) => continue,
                Edit::Insert(&value) => {
                    c.insert(i, value);
                }
                Edit::Remove(_) => {
                    c.remove(i);
                }
                Edit::Replace(&value) => c[i] = value,
            }
        }

        assert_eq!(b, c);
    }
}
