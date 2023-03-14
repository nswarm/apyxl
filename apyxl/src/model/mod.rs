#[derive(Default, Debug, Clone)]
pub struct Model {
    pub dtos: Vec<Dto>,
}

#[derive(Default, Debug, Clone)]
pub struct Dto {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Default, Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: String, // todo DtoRef?
}
