#[derive(Debug)]
pub struct Metadata {
    primitives: Primitives,
}

#[derive(Debug)]
pub struct Primitives {
    int8: Option<String>,
    int16: Option<String>,
    int32: Option<String>,
    int64: Option<String>,
    int128: Option<String>,

    uint8: Option<String>,
    uint16: Option<String>,
    uint32: Option<String>,
    uint64: Option<String>,
    uint128: Option<String>,

    float8: Option<String>,
    float16: Option<String>,
    float32: Option<String>,
    float64: Option<String>,
    float128: Option<String>,
}

fn asdf() {}
