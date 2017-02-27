use json::JsonValue;

use KintoClient;
use error::KintoError;
use paths::Paths;
use request::{GetCollection, DeleteCollection, GetRecord, CreateRecord,
              UpdateRecord, DeleteRecord, KintoRequest};
use response::ResponseWrapper;
use resource::Resource;
use collection::Collection;
use utils::{unwrap_collection_ids, format_permissions};


#[derive(Debug, Clone)]
pub struct BucketPermissions {
    pub read: Vec<String>,
    pub write: Vec<String>,
    pub create_collection: Vec<String>,
    pub create_group: Vec<String>,
}


impl From<JsonValue> for BucketPermissions {
    fn from(json: JsonValue) -> Self {
        BucketPermissions {
            read: format_permissions(json["read"]
                                     .to_owned()),
            write: format_permissions(json["write"]
                                      .to_owned()),
            create_collection: format_permissions(json["create:collection"]
                                                  .to_owned()),
            create_group: format_permissions(json["create:group"]
                                             .to_owned())
        }
    }
}


impl Into<JsonValue> for BucketPermissions {
    fn into(self) -> JsonValue {
        let mut obj = JsonValue::new_object();
        obj["read"] = self.read.into();
        obj["write"] = self.write.into();
        obj["collection:create"] = self.create_collection.into();
        obj["group:create"] = self.create_group.into();
        return obj;
    }
}


#[derive(Debug, Clone)]
pub struct Bucket {
    pub client: KintoClient,
    pub id: String,
    pub timestamp: Option<u64>,
    pub data: Option<JsonValue>,
    pub permissions: Option<BucketPermissions>,
}


impl Bucket {
    /// Create a new bucket resource.
    pub fn new<'a>(client: KintoClient, id: &'a str) -> Self {
        Bucket {client: client, id: id.to_owned(),
                timestamp: None, data: None, permissions: None}
    }

    pub fn collection(self, id: &'static str) -> Collection {
        return Collection::new(self.client.clone(), self, id);
    }

    /// Create a new empty collection with a generated id.
    pub fn new_collection(&mut self) -> Result<Collection, KintoError> {
        match self.create_collection_request().send() {
            Ok(wrapper) => Ok(wrapper.into()),
            Err(value) => return Err(value)
        }
    }

    /// List the names of all available collections.
    pub fn list_collections(&mut self) -> Result<Vec<String>, KintoError> {
        let response = try!(self.list_collections_request().send());
        // XXX: we should follow possible subrequests
        Ok(unwrap_collection_ids(response))
    }

    /// Delete all available collections.
    pub fn delete_collections(&mut self) -> Result<(), KintoError> {
        try!(self.delete_collections_request().send());
        Ok(())
    }

    /// Create a custom list collections request.
    pub fn list_collections_request(&mut self) -> GetCollection {
        GetCollection::new(self.client.clone(),
                           Paths::Collections(self.id.as_str()).into())
    }

    /// Create a custom delete collections request.
    pub fn delete_collections_request(&mut self) -> DeleteCollection {
        DeleteCollection::new(self.client.clone(),
                              Paths::Collections(self.id.as_str()).into())
    }

    /// Create a custom create collection request.
    pub fn create_collection_request(&mut self) -> CreateRecord {
        CreateRecord::new(self.client.clone(),
                          Paths::Collections(self.id.as_str()).into())
    }
}


impl Resource for Bucket {
    fn unwrap_response(&mut self, wrapper: ResponseWrapper){
        *self = wrapper.into()
    }

    fn get_data(&mut self) ->  Option<JsonValue> {
        self.data.clone()
    }

    fn get_permissions(&mut self) ->  Option<JsonValue> {
        match self.permissions.clone() {
            Some(perms) => Some(perms.into()),
            None => None
        }
    }

    fn get_timestamp(&mut self) -> Option<u64> {
        self.timestamp
    }

    fn load_request(&mut self) -> GetRecord {
        GetRecord::new(self.client.clone(),
                       Paths::Bucket(self.id.as_str()).into())
    }

    fn update_request(&mut self) -> UpdateRecord {
        UpdateRecord::new(self.client.clone(),
                          Paths::Bucket(self.id.as_str()).into())
    }

    fn delete_request(&mut self) -> DeleteRecord {
        DeleteRecord::new(self.client.clone(),
                          Paths::Bucket(self.id.as_str()).into())
    }
}


impl From<ResponseWrapper> for Bucket {
    fn from(wrapper: ResponseWrapper) -> Self {
        let timestamp = wrapper.json["data"]["last_modified"].as_number().unwrap();
        Bucket {
            client: wrapper.client,
            data: wrapper.json["data"].to_owned().into(),
            permissions: Some(wrapper.json["permissions"].to_owned().into()),
            id: wrapper.json["data"]["id"].to_string(),
            timestamp: Some(timestamp.into())
        }
    }
}


impl Into<JsonValue> for Bucket {
    fn into(self) -> JsonValue {
        let mut obj = JsonValue::new_object();
        match self.data {
            Some(data) => obj["data"] = data.into(),
            None => ()
        }
        match self.permissions {
            Some(perms) => obj["permissions"] = perms.into(),
            None => ()
        }
        return obj;
    }
}


#[cfg(test)]
mod test_bucket {
    use utils::tests::{setup_client, setup_bucket};
    use resource::Resource;

    #[test]
    fn test_create_bucket() {
        let mut bucket = setup_bucket();
        bucket.data = object!{"good" => true}.into();

        bucket.create().unwrap();
        let data = bucket.data.unwrap().to_owned();

        assert_eq!(data["id"], "food");
        assert_eq!(data["good"], true);
    }

    #[test]
    fn test_create_bucket_fails_on_existing() {
        let mut bucket = setup_bucket();

        // Create
        bucket.create().unwrap();

        // Tries to create again
        match bucket.create() {
            Ok(_) => panic!(""),
            Err(_) => ()
        }
    }

    #[test]
    fn test_load_bucket() {
        let mut bucket = setup_bucket();
        bucket.set().unwrap();
        let create_data = bucket.data.clone().unwrap();

        // Cleanup stored data to make sure load work
        bucket.data = object!{}.into();

        bucket.load().unwrap();
        let load_data = bucket.data.unwrap();


        assert_eq!(create_data, load_data);
    }

    #[test]
    fn test_load_bucket_fails_on_not_existing() {
        let mut bucket = setup_bucket();
        match bucket.load() {
            Ok(_) => panic!(""),
            Err(_) => ()
        }
    }

    #[test]
    fn test_update_bucket() {
        let mut bucket = setup_bucket();

        bucket.create().unwrap();
        let create_data = bucket.data.clone().unwrap();

        bucket.update().unwrap();
        let update_data = bucket.data.unwrap();

        assert_eq!(create_data["id"], update_data["id"]);
        assert!(create_data["last_modified"] != update_data["last_modified"]);
    }

    #[test]
    fn test_update_bucket_fails_on_not_existing() {
        let mut client = setup_client();
        let mut bucket = client.bucket("food");
        match bucket.update() {
            Ok(_) => panic!(""),
            Err(_) => ()
        }
    }

    #[test]
    fn test_get_collection() {
        let bucket = setup_bucket();
        let collection = bucket.collection("meat");
        assert_eq!(collection.id, "meat");
        assert!(collection.data == None);
    }

    #[test]
    fn test_new_collection() {
        let mut bucket = setup_bucket();
        bucket.create().unwrap();
        let collection = bucket.new_collection().unwrap();
        assert!(collection.data != None);
        assert_eq!(collection.id, collection.data.unwrap()["id"].to_string());
    }

    #[test]
    fn test_list_collections() {
        let mut bucket = setup_bucket();
        bucket.create().unwrap();
        assert_eq!(bucket.list_collections().unwrap().len(), 0);
        bucket.new_collection().unwrap();
        assert_eq!(bucket.list_collections().unwrap().len(), 1);
    }

    #[test]
    fn test_delete_collections() {
        let mut bucket = setup_bucket();
        bucket.create().unwrap();
        bucket.new_collection().unwrap();
        assert_eq!(bucket.list_collections().unwrap().len(), 1);
        bucket.delete_collections().unwrap();
        assert_eq!(bucket.list_collections().unwrap().len(), 0);
    }

    #[test]
    fn test_list_collections_request() {
        let mut bucket = setup_bucket();
        let request = bucket.list_collections_request();
        assert_eq!(request.preparer.path, "/buckets/food/collections");
    }

    #[test]
    fn test_delete_collections_request() {
        let mut bucket = setup_bucket();
        let request = bucket.delete_collections_request();
        assert_eq!(request.preparer.path, "/buckets/food/collections");
    }

    #[test]
    fn test_create_collection_request() {
        let mut bucket = setup_bucket();
        let request = bucket.create_collection_request();
        assert_eq!(request.preparer.path, "/buckets/food/collections");
    }
}