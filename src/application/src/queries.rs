use uuid::Uuid;

pub struct TrackOrderQuery {
    tracking_id: Uuid,
}

impl TrackOrderQuery {
    pub fn new(tracking_id: Uuid) -> Self {
        Self { tracking_id }
    }

    pub fn tracking_id(&self) -> Uuid {
        self.tracking_id
    }
}
