use crate::error::{Result, WorkVcsError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};
use uuid::Uuid;

macro_rules! typed_uuid {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new_v7() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn from_uuid(uuid: Uuid) -> Result<Self> {
                require_uuidv7(uuid)?;
                Ok(Self(uuid))
            }

            pub fn from_bytes(bytes: [u8; 16]) -> Result<Self> {
                Self::from_uuid(Uuid::from_bytes(bytes))
            }

            pub fn as_uuid(&self) -> Uuid {
                self.0
            }

            pub fn raw_bytes(&self) -> [u8; 16] {
                *self.0.as_bytes()
            }

            pub fn parse_canonical(input: &str) -> Result<Self> {
                let uuid = Uuid::parse_str(input).map_err(|error| {
                    WorkVcsError::IdentityInvalid(format!("invalid UUID: {error}"))
                })?;
                let canonical = uuid.hyphenated().to_string();
                if input != canonical {
                    return Err(WorkVcsError::IdentityInvalid(
                        "UUID text must be lowercase hyphenated canonical form".to_owned(),
                    ));
                }
                require_uuidv7(uuid)?;
                Ok(Self(uuid))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.raw_bytes().cmp(&other.raw_bytes())
            }
        }

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0.hyphenated().to_string())
            }
        }

        impl FromStr for $name {
            type Err = WorkVcsError;

            fn from_str(input: &str) -> Result<Self> {
                Self::parse_canonical(input)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse_canonical(&value).map_err(serde::de::Error::custom)
            }
        }
    };
}

fn require_uuidv7(uuid: Uuid) -> Result<()> {
    if uuid.get_version_num() == 7 {
        Ok(())
    } else {
        Err(WorkVcsError::IdentityInvalid(
            "UUID must be version 7".to_owned(),
        ))
    }
}

typed_uuid!(StoreId);
typed_uuid!(WorkspaceId);
typed_uuid!(EntityId);
typed_uuid!(EntityVersionId);
typed_uuid!(EvidenceId);
typed_uuid!(ResourceId);
typed_uuid!(ResourceObservationId);
typed_uuid!(RelationId);
typed_uuid!(RelationVersionId);
typed_uuid!(BranchId);
typed_uuid!(CommitId);
typed_uuid!(ChangeSetId);
typed_uuid!(OperationId);
typed_uuid!(EventId);
typed_uuid!(SessionId);
typed_uuid!(SessionDiffId);
typed_uuid!(ClaimId);
typed_uuid!(MergeId);
