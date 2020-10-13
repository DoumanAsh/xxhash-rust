const DATA: [&str; 35] = [
    "waifulandshigtgsqwetyuop[]asbnm,./",
    "waifulandshigtgsqwetyuop[]asbnm,.",
    "waifulandshigtgsqwetyuop[]asbnm,",
    "waifulandshigtgsqwetyuop[]asbnm",
    "waifulandshigtgsqwetyuop[]asbn",
    "waifulandshigtgsqwetyuop[]asb",
    "waifulandshigtgsqwetyuop[]as",
    "waifulandshigtgsqwetyuop[]a",
    "waifulandshigtgsqwetyuop[]",
    "waifulandshigtgsqwetyuop[",
    "waifulandshigtgsqwetyuop",
    "waifulandshigtgsqwetyuo",
    "waifulandshigtgsqwetyu",
    "waifulandshigtgsqwety",
    "waifulandshigtgsqwet",
    "waifulandshigtgsqwe",
    "waifulandshigtgsqw",
    "waifulandshigtgsq",
    "waifulandshigtgs",
    "waifulandshigtg",
    "waifulandshigt",
    "waifulandshig",
    "waifulandshi",
    "waifulandsh",
    "waifulands",
    "waifuland",
    "waifulan",
    "waifula",
    "waiful",
    "lolka",
    "lolk",
    "lol",
    "lo",
    "l",
    "",
];

#[cfg(feature = "xxh32")]
#[test]
fn assert_v32() {
    use xxhash_c_sys as sys;
    use xxhash_rust::xxh32::xxh32;

    const SEED_1: u32 = 0;
    const SEED_2: u32 = 1;

    let mut hasher_1 = xxhash_rust::xxh32::Xxh32::new(SEED_1);
    let mut hasher_2 = xxhash_rust::xxh32::Xxh32::new(SEED_2);

    for input in DATA.iter() {
        println!("input(len={})='{}'", input.len(), input);
        let sys_result = unsafe {
            sys::XXH32(input.as_ptr() as _, input.len(), SEED_1)
        };
        let result = xxh32(input.as_bytes(), SEED_1);
        assert_eq!(result, sys_result);
        hasher_1.update(input.as_bytes());
        assert_eq!(hasher_1.finish(), result);

        let sys_result = unsafe {
            sys::XXH32(input.as_ptr() as _, input.len(), SEED_2)
        };
        let result = xxh32(input.as_bytes(), SEED_2);
        assert_eq!(result, sys_result);
        hasher_2.update(input.as_bytes());
        assert_eq!(hasher_2.finish(), result);

        hasher_1.reset(SEED_1);
        hasher_2.reset(SEED_2);
    }
}

#[cfg(feature = "const_xxh32")]
#[test]
fn assert_v32() {
    use xxhash_c_sys as sys;
    use xxhash_rust::const_xxh32::xxh32;

    const SEED_1: u32 = 0;
    const SEED_2: u32 = 1;

    for input in DATA.iter().rev() {
        println!("input(len={})='{}'", input.len(), input);
        let sys_result = unsafe {
            sys::XXH32(input.as_ptr() as _, input.len(), SEED_1)
        };
        let result = xxh32(input.as_bytes(), SEED_1);
        assert_eq!(result, sys_result);

        let sys_result = unsafe {
            sys::XXH32(input.as_ptr() as _, input.len(), SEED_2)
        };
        let result = xxh32(input.as_bytes(), SEED_2);
        assert_eq!(result, sys_result);
    }
}
