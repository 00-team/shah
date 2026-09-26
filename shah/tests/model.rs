#[cfg(test)]
mod tests {
    use shah::models::Gene;

    #[shah::model]
    #[derive(Debug)]
    struct ShahUser {
        gene: Gene,
        array: [[u8; 200]; 200],
    }

    #[test]
    fn test() {
        let user = ShahUser::default();
        assert_eq!(user.array.len(), 200);
        assert_eq!(user.array[0].len(), 200);
    }
}
