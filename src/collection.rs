use json::JsonValue;

use KintoClient;
use error::KintoError;
use paths::Paths;
use request::{GetCollection, DeleteCollection, GetRecord, CreateRecord,
              UpdateRecord, DeleteRecord, KintoRequest};
use response::ResponseWrapper;
use resource::Resource;
use bucket::Bucket;
use record::Record;
use utils::{unwrap_collection_ids, extract_ids_from_path};


#[derive(Debug, Clone)]
pub struct CollectionPermissions {
    pub read: Vec<String>,
    pub write: Vec<String>,
    pub create_record: Vec<String>,
}


#[derive(Debug, Clone)]
pub struct Collection {
    pub client: KintoClient,
    pub bucket: Bucket,
    pub id: String,
    pub timestamp: Option<u64>,
    pub data: Option<JsonValue>,
    pub permissions: Option<JsonValue>,
}


impl Collection {

    /// Create a new collection resource.
    pub fn new<'a>(client: KintoClient, bucket: Bucket, id: &'a str) -> Self {
        Collection {client: client, bucket: bucket, id: id.to_owned(),
                    timestamp: None, data: None, permissions: None}
    }

    pub fn record(self, id: &'static str) -> Record {
        return Record::new(self.client.clone(), self, id);
    }

    /// Create a new empty record with a generated id.
    pub fn new_record(&mut self) -> Result<Record, KintoError> {
        match self.create_record_request().send() {
            Ok(wrapper) => Ok(wrapper.into()),
            Err(value) => return Err(value)
        }
    }

    /// List the names of all available records.
    pub fn list_records(&mut self) -> Result<Vec<String>, KintoError> {
        let response = try!(self.list_records_request().send());
        // XXX: we should follow possible subrequests
        Ok(unwrap_collection_ids(response))
    }

    /// Delete all available records.
    pub fn delete_records(&mut self) -> Result<(), KintoError> {
        try!(self.delete_records_request().send());
        Ok(())
    }

    pub fn list_records_request(&mut self) -> GetCollection {
        GetCollection::new(self.client.clone(),
                           Paths::Records(self.bucket.id.as_str(),
                                          self.id.as_str()).into())
    }

    pub fn delete_records_request(&mut self) -> DeleteCollection {
        DeleteCollection::new(self.client.clone(),
                           Paths::Records(self.bucket.id.as_str(),
                                          self.id.as_str()).into())
    }

    pub fn create_record_request(&mut self) -> CreateRecord {
        CreateRecord::new(self.client.clone(),
                           Paths::Records(self.bucket.id.as_str(),
                                          self.id.as_str()).into())
    }
}


impl Resource for Collection {

    fn unwrap_response(&mut self, wrapper: ResponseWrapper){
        *self = wrapper.into()
    }

    fn get_data(&mut self) ->  Option<JsonValue> {
        self.data.clone()
    }

    fn get_permissions(&mut self) ->  Option<JsonValue> {
        self.permissions.clone()
    }

    fn get_timestamp(&mut self) -> Option<u64> {
        self.timestamp
    }

    fn load_request(&mut self) -> GetRecord {
        GetRecord::new(self.client.clone(),
                       Paths::Collection(self.bucket.id.as_str(),
                                         self.id.as_str()).into())
    }

    fn update_request(&mut self) -> UpdateRecord {
        UpdateRecord::new(self.client.clone(),
                          Paths::Collection(self.bucket.id.as_str(),
                                            self.id.as_str()).into())
    }

    fn delete_request(&mut self) -> DeleteRecord {
        DeleteRecord::new(self.client.clone(),
                          Paths::Collection(self.bucket.id.as_str(),
                                            self.id.as_str()).into())
    }
}


impl From<ResponseWrapper> for Collection {
    fn from(wrapper: ResponseWrapper) -> Self {
        let timestamp = wrapper.json["data"]["last_modified"]
                                .as_number().unwrap();

        let path_ids = extract_ids_from_path(wrapper.path);
        let bucket_id = path_ids["buckets"].clone().unwrap();

        Collection {
            client: wrapper.client.clone(),
            bucket: Bucket::new(wrapper.client, bucket_id.as_str()),
            data: wrapper.json["data"].to_owned().into(),
            permissions: wrapper.json["permissions"].to_owned()
                                                    .into(),
            id: wrapper.json["data"]["id"].to_string(),
            timestamp: Some(timestamp.into())
        }
    }
}


impl Into<JsonValue> for Collection {
    fn into(self) -> JsonValue {
        let mut obj = JsonValue::new_object();
        match self.data {
            Some(data) => obj["data"] = data,
            None => ()
        }
        match self.permissions {
            Some(perms) => obj["permissions"] = perms,
            None => ()
        }
        return obj;
    }
}


#[cfg(test)]
mod test_client {
    use resource::Resource;
    use utils::tests::{setup_collection, setup_bucket};

    #[test]
    fn test_create_collection() {
        let mut collection = setup_collection();
        collection.data = object!{"good" => true}.into();

        collection.create().unwrap();
        let data = collection.data.unwrap().to_owned();

        assert_eq!(data["id"], "meat");
        assert_eq!(data["good"], true);
    }

    #[test]
    fn test_create_collection_fails_on_existing() {
        let mut collection = setup_collection();

        // Create
        collection.create().unwrap();

        // Tries to create again
        match collection.create() {
            Ok(_) => panic!(""),
            Err(_) => ()
        }
    }

    #[test]
    fn test_load_collection() {
        let mut collection = setup_collection();
        collection.set().unwrap();
        let create_data = collection.data.clone().unwrap();

        // Cleanup stored data to make sure load work
        collection.data = object!{}.into();

        collection.load().unwrap();
        let load_data = collection.data.unwrap();


        assert_eq!(create_data, load_data);
    }

    #[test]
    fn test_load_collection_fails_on_not_existing() {
        let mut collection = setup_collection();
        match collection.load() {
            Ok(_) => panic!(""),
            Err(_) => ()
        }
    }

    #[test]
    fn test_update_collection() {
        let mut collection = setup_collection();

        collection.create().unwrap();
        let create_data = collection.data.clone().unwrap();

        collection.update().unwrap();
        let update_data = collection.data.unwrap();

        assert_eq!(create_data["id"], update_data["id"]);
        assert!(create_data["last_modified"] != update_data["last_modified"]);
    }

    #[test]
    fn test_update_collection_fails_on_not_existing() {
        let client = setup_bucket();
        let mut collection = client.collection("food");
        match collection.update() {
            Ok(_) => panic!(""),
            Err(_) => ()
        }
    }

    #[test]
    fn test_get_record() {
        let collection = setup_collection();
        let record = collection.record("entrecote");
        assert_eq!(record.id, "entrecote");
        assert!(record.data == None);
    }

    #[test]
    fn test_new_record() {
        let mut collection = setup_collection();
        collection.create().unwrap();
        let record = collection.new_record().unwrap();
        assert!(record.data != None);
        assert_eq!(record.id, record.data.unwrap()["id"].to_string());
    }

    #[test]
    fn test_list_records() {
        let mut collection = setup_collection();
        collection.create().unwrap();
        assert_eq!(collection.list_records().unwrap().len(), 0);
        collection.new_record().unwrap();
        assert_eq!(collection.list_records().unwrap().len(), 1);
    }

    #[test]
    fn test_delete_records() {
        let mut collection = setup_collection();
        collection.create().unwrap();
        collection.new_record().unwrap();
        assert_eq!(collection.list_records().unwrap().len(), 1);
        collection.delete_records().unwrap();
        assert_eq!(collection.list_records().unwrap().len(), 0);
    }

    #[test]
    fn test_list_records_request() {
        let mut collection = setup_collection();
        let request = collection.list_records_request();
        assert_eq!(request.preparer.path, "/buckets/food/collections/meat/records");
    }

    #[test]
    fn test_delete_records_request() {
        let mut collection = setup_collection();
        let request = collection.delete_records_request();
        assert_eq!(request.preparer.path, "/buckets/food/collections/meat/records");
    }

    #[test]
    fn test_create_records_request() {
        let mut collection = setup_collection();
        let request = collection.create_record_request();
        assert_eq!(request.preparer.path, "/buckets/food/collections/meat/records");
    }
}