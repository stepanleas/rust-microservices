use uuid::Uuid;

pub struct CreateCategoryCommand<'a> {
    title: &'a str,
    description: &'a str,
}

impl<'a> CreateCategoryCommand<'a> {
    pub fn new(title: &'a str, description: &'a str) -> Self {
        CreateCategoryCommand { title, description }
    }

    pub fn title(&self) -> &str {
        self.title
    }

    pub fn description(&self) -> &str {
        self.description
    }
}

pub struct UpdateCategoryCommand<'a> {
    id: Uuid,
    title: &'a str,
    description: &'a str,
}

impl<'a> UpdateCategoryCommand<'a> {
    pub fn new(id: Uuid, title: &'a str, description: &'a str) -> Self {
        UpdateCategoryCommand {
            id,
            title,
            description,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn title(&self) -> &str {
        self.title
    }

    pub fn description(&self) -> &str {
        self.description
    }
}

pub struct DeleteCategoryCommand {
    id: Uuid,
}

impl DeleteCategoryCommand {
    pub fn new(id: Uuid) -> Self {
        DeleteCategoryCommand { id }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}
