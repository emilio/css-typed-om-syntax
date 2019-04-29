

/// Trims ascii whitespace characters from a slice, and returns the trimmed
/// input.
pub fn trim_ascii_whitespace(input: &[u8]) -> &[u8] {
    if input.is_empty() {
        return input;
    }

    let mut start = 0;
    {
        let mut iter = input.iter();
        loop {
            let byte = match iter.next() {
                Some(b) => b,
                None => return &[],
            };

            if !byte.is_ascii_whitespace() {
                break;
            }
            start += 1;
        }
    }

    let mut end = input.len();
    assert!(start < end);
    {
        let mut iter = input[start..].iter().rev();
        loop {
            let byte = match iter.next() {
                Some(b) => b,
                None => {
                    debug_assert!(false, "We should have caught this in the loop above!");
                    return &[];
                },
            };

            if !byte.is_ascii_whitespace() {
                break;
            }
            end -= 1;
        };
    }

    &input[start..end]
}

#[test]
fn trim_ascii_whitespace_test() {
    fn test(i: &str, o: &str) {
        assert_eq!(
            trim_ascii_whitespace(i.as_bytes()),
            o.as_bytes(),
        )
    }

    test("", "");
    test(" ", "");
    test(" a b c ", "a b c");
    test(" \t \t \ta b c \t \t \t \t", "a b c");
}
