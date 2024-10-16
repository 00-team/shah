#[test]
fn test_str_get_set() {
    #[shah_macros::model]
    struct SomeModel {
        #[str]
        name: [u8; 10],
    }

    let mut model = SomeModel::default();

    // test empty init
    assert_eq!(model.name(), "");
    assert_eq!(model.name, [0u8; 10]);

    // test normal ascii
    model.set_name("this is");
    assert_eq!(model.name(), "this is");
    assert_eq!(model.name, [116, 104, 105, 115, 32, 105, 115, 0, 0, 0]);

    // test valid character boundary
    model.set_name("this c 🐧");
    assert_eq!(model.name(), "this c ");
    assert_eq!(model.name, [116, 104, 105, 115, 32, 99, 32, 0, 0, 0]);

    // test normal emoji
    model.set_name("gg 🐧");
    assert_eq!(model.name(), "gg 🐧");
    assert_eq!(model.name, [103, 103, 32, 240, 159, 144, 167, 0, 0, 0]);
}
