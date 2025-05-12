pub fn generate_strong_password(_len: usize) -> String {
    use rand::{thread_rng, Rng};

    let charset: Vec<char> = (33u8..=126u8)
        .filter(|&c| c != b'-')
        .map(|c| c as char)
        .collect();

    let mut rng = thread_rng();
    let mut password = String::new();
    let total_chars = 20;

    let mut prev_char: Option<char> = None;

    for i in 0..total_chars {
        let mut c;
        loop {
            c = charset[rng.gen_range(0..charset.len())];
            if let Some(prev) = prev_char {
                if (prev as i16 - c as i16).abs() <= 1 { // No neighbor characters in ASCII
                    continue;
                }
            }
            break;
        }

        password.push(c);
        prev_char = Some(c);

        if i == 4 || i == 9 || i == 14 {
            password.push('-');
            prev_char = Some('-');
        }
    }

    password
}