use super::*;
use url::Url;

#[test]
fn official_management_routes_encode_each_path_segment() {
    let endpoint = management_endpoint(
        &Url::parse("https://img.example.com/upload").unwrap(),
        "rename",
        "blog/中文 图.png",
    )
    .unwrap();
    assert_eq!(
        endpoint.as_str(),
        "https://img.example.com/api/manage/rename/blog/%E4%B8%AD%E6%96%87%20%E5%9B%BE.png"
    );
}

#[test]
fn paths_reject_traversal_and_rename_keeps_the_directory() {
    assert!(management_endpoint(
        &Url::parse("https://img.example.com").unwrap(),
        "move",
        "../secret"
    )
    .is_err());
    assert_eq!(
        CloudflareImgbedClient::renamed_path("blog/old.png", "new.png"),
        "blog/new.png"
    );
}
