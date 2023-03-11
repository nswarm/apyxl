#[derive(Default, Debug)]
pub struct Model {
    pub dtos: Vec<Dto>,
}

#[derive(Default, Debug)]
pub struct Dto {
    pub name: String,
}
