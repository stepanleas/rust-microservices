mod api_doc;
mod customer_api_doc;
mod health_check_api_doc;
mod info_api_doc;

pub use api_doc::open_api_docs;

#[cfg(test)]
mod api_doc_tests;
