use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(Uuid);

impl EntityId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn uuid(&self) -> Uuid {
        self.0
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Entity({})", &self.0.to_string()[..8]) // Show first 8 chars
    }
}

pub struct EntityManager {
    alive_entities: std::collections::HashSet<EntityId>,
    next_generation: u32, // For debugging/metrics
}

impl EntityManager {
    pub fn new() -> Self {
        Self {
            alive_entities: std::collections::HashSet::new(),
            next_generation: 0,
        }
    }

    pub fn create_entity(&mut self) -> EntityId {
        let entity = EntityId::new();
        self.alive_entities.insert(entity);
        self.next_generation += 1;

        println!("🆕 Created entity: {}", entity);
        entity
    }

    pub fn destroy_entity(&mut self, entity: EntityId) -> bool {
        let was_alive = self.alive_entities.remove(&entity);
        was_alive
    }

    pub fn is_alive(&self, entity: EntityId) -> bool {
        self.alive_entities.contains(&entity)
    }

    pub fn get_alive_entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.alive_entities.iter().copied()
    }

    pub fn entity_count(&self) -> usize {
        self.alive_entities.len()
    }

    pub fn generation(&self) -> u32 {
        self.next_generation
    }
}
