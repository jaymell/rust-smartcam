use sea_orm::{entity::prelude::*};

#[derive(Debug, Clone, DeriveEntityModel)]
#[sea_orm(table_name = "video_file")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub path: String,
    pub name: String,
    pub label: String,
    pub ts: DateTimeUtc,
    pub created: DateTimeUtc,
    pub modified: DateTimeUtc,
    pub storage_type_id: i32,
}

#[derive(Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::storage_type::Entity")]
    StorageType
}

impl Related<super::storage_type::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StorageType.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
//     fn new() -> Self {
//         Self {
//             id: Set(EndpointId::new(None, None)),
//             created_at: Set(Utc::now().into()),
//             updated_at: Set(Utc::now().into()),
//             deleted: Set(false),
//             ..ActiveModelTrait::default()
//         }
//     }

//     fn before_save(mut self, _insert: bool) -> Result<Self, DbErr> {
//         self.updated_at = Set(Utc::now().into());
//         Ok(self)
//     }
// }
