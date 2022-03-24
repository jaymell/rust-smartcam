use sea_orm::{entity::prelude::*};

#[derive(Debug, Clone, DeriveEntityModel)]
#[sea_orm(table_name = "storage_type")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
}

#[derive(Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::video_file::Entity")]
    VideoFile 
}

impl Related<super::video_file::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VideoFile.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
    pub fn find_by_name(name: &str) -> Select<Self> {
        Self::find().filter(Column::Name.eq(name))
    }
}