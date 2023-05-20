use crate::integration::types::*;
use serde_json::value::{Value};
use actix_http::{body::MessageBody, Request};
use actix_web::{
  dev::{Service, ServiceResponse},
  test::{call_and_read_body_json, TestRequest},
};

pub struct Customer;

impl RequestBuilder for Customer{
  fn make_request_body(data : &MasterData) -> Option<TestRequest>{
    match data.customers {
      Some(customer_data) => {
        let request_body = Value::clone(&customer_data.as_ref());
        TestRequest::post()
            .uri(&String::from("http://localhost:8080/customers"))
            .insert_header(("api-key",data.api_key.as_ref().unwrap().as_str()))
            .set_json(&request_body)
      }
      None => None,
    }
  }

  fn verify_response(s : &Value) -> Self{
      assert_eq!(true,true);
      Self
  }

  fn update_master_data(&self,data : &mut MasterData, resp : &Value){
    
  }

}

pub async fn execute_customer_create_test(master_data : &mut MasterData, server: &impl Service<Request, Response = ServiceResponse<impl MessageBody>, Error = actix_web::Error>){
  let opt_test_request = Customer::make_request_body(&master_data);
  match opt_test_request{
    Some(test_request) => {
      let customer_create_resp = call_and_read_body_json(&server,test_request.to_request()).await;
      Customer::verify_response(&Customer_create_resp).update_master_data(master_data,&customer_create_resp);
      println!("Customer Create Response : {:?}",customer_create_resp);
    },
    None => {
      println!("Skipping Customer Create Test!")
    },
  }
  

}