use roxmltree::Node;

use crate::clients::plex_client::{PlexClientError, PlexClientResult};

#[derive(Debug)]
pub struct GetResources {
    pub devices: Vec<Device>,
}

#[derive(Debug)]
pub struct Device {
    pub name: String,
    pub product: String,
    pub provides: String,
    pub client_identifier: String,
}

impl GetResources {
    pub fn from_xml(xml: &str) -> PlexClientResult<Self> {
        let doc = roxmltree::Document::parse(xml)?;
        let device_node = doc.descendants().find(Self::is_device_node).ok_or(
            PlexClientError::MisunderstoodPlexResponse(
                "could not find Device node in get resource respones".to_string(),
            ),
        )?;

        let devices = device_node
            .next_siblings()
            .filter_map(Self::build_device)
            .collect::<Vec<_>>();
        Ok(Self { devices })
    }

    fn is_device_node(node: &Node) -> bool {
        node.tag_name().name() == "Device"
    }

    fn build_device(node: Node) -> Option<Device> {
        if !Self::is_device_node(&node) {
            return None;
        }
        let device = Device {
            name: node.attribute("name")?.to_string(),
            product: node.attribute("product")?.to_string(),
            provides: node.attribute("provides")?.to_string(),
            client_identifier: node.attribute("clientIdentifier")?.to_string(),
        };
        Some(device)
    }
}

#[derive(Debug)]
pub struct GetUserInfo {
    pub users: Vec<User>,
}

#[derive(Debug)]
pub struct User {
    pub username: String,
    pub user_id: String,
    pub access_token: String,
}

impl GetUserInfo {
    pub fn from_xml(xml: &str) -> PlexClientResult<Self> {
        let doc = roxmltree::Document::parse(xml)?;
        let device_node = doc.descendants().find(Self::is_user_node).ok_or(
            PlexClientError::MisunderstoodPlexResponse(
                "could not find User node in get resource respones".to_string(),
            ),
        )?;
        let users = device_node
            .next_siblings()
            .filter_map(Self::build_user)
            .collect::<Vec<_>>();
        Ok(Self { users })
    }

    fn is_user_node(node: &Node) -> bool {
        node.tag_name().name() == "SharedServer"
    }

    fn build_user(node: Node) -> Option<User> {
        if !Self::is_user_node(&node) {
            return None;
        }
        let user = User {
            username: node.attribute("username")?.to_string(),
            user_id: node.attribute("userID")?.to_string(),
            access_token: node.attribute("accessToken")?.to_string(),
        };
        Some(user)
    }
}
