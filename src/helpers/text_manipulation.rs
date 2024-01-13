use regex::Regex;

pub enum Direction {
    Left,
    //Right,
}

pub fn rotate(str: &str, direction: Direction, count: usize) -> String {
    let mut str_vec: Vec<char> = str.chars().collect();
    match direction {
        Direction::Left => str_vec.rotate_left(count),
        //Direction::Right => str_vec.rotate_right(count),
    }
    str_vec.iter().collect()
}

pub fn humanize_string(input: &str) -> String {
    let snake_case_regex = Regex::new(r"_(.)").unwrap();
    let camel_case_regex = Regex::new(r"([a-z])([A-Z])").unwrap();

    let snake_case_result = snake_case_regex.replace_all(input, |caps: &regex::Captures| {
        format!(" {}", caps[1].to_uppercase())
    });

    let camel_case_result = camel_case_regex.replace_all(&snake_case_result, "$1 $2");

    let title_case_regex = Regex::new(r"(^|\s)(\p{Ll})").unwrap();
    let result = title_case_regex.replace_all(&camel_case_result, |caps: &regex::Captures| {
        format!("{}{}", &caps[1], caps[2].to_uppercase())
    });

    let filtered_result = result
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>();

    filtered_result
}

pub fn determine_field_value(input: &str) -> String {
    if input.to_string() == "0" {
        return "".into();
    } else {
        let val: String = input
            .to_string()
            .chars()
            .filter(|c| c.is_numeric())
            .collect();

        let parsed_val = val.parse().unwrap_or(0);

        parsed_val.min(65535).to_string()
    }
}
