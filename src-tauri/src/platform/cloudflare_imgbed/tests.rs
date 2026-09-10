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

#[tokio::test]
async fn stalled_requests_have_a_finite_budget_and_mutation_timeout_guidance() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.unwrap();
        std::future::pending::<()>().await;
    });
    let client = http_client_builder()
        .timeout(std::time::Duration::from_millis(50))
        .build()
        .unwrap();
    let error = client
        .get(format!("http://{address}"))
        .send()
        .await
        .unwrap_err();
    let result = request_error("upload_failed", error, true);
    assert_eq!(result.code, "imgbed_request_timeout");
    assert!(result.message.contains("结果尚未确认"));
    server.abort();
}
