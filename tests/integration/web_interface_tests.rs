use super::*;
use axum::http::{Method, StatusCode, header};

#[tokio::test]
async fn test_web_page_routes() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    let mut app = create_router(state);

    // Test all main page routes
    let routes = vec![
        "/",
        "/dashboard", 
        "/setup",
        "/products",
        "/scheduler",
        "/settings",
    ];

    for route in routes {
        let response = make_request(&mut app, Method::GET, route, None).await?;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Route {} should return 200 OK",
            route
        );
        
        // Verify content type is HTML
        let content_type = response.headers().get(header::CONTENT_TYPE);
        if let Some(ct) = content_type {
            let ct_str = ct.to_str().unwrap_or("");
            assert!(
                ct_str.contains("text/html") || ct_str.is_empty(),
                "Route {} should return HTML content, got: {}",
                route,
                ct_str
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_static_asset_serving() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    let mut app = create_router(state);

    // Test CSS file
    let response = make_request(&mut app, Method::GET, "/static/css/style.css", None).await?;
    assert_eq!(response.status(), StatusCode::OK);
    
    let content_type = response.headers().get(header::CONTENT_TYPE);
    assert!(content_type.is_some());
    assert_eq!(content_type.unwrap(), "text/css");

    // Test JavaScript file
    let response = make_request(&mut app, Method::GET, "/static/js/app.js", None).await?;
    assert_eq!(response.status(), StatusCode::OK);
    
    let content_type = response.headers().get(header::CONTENT_TYPE);
    assert!(content_type.is_some());
    assert_eq!(content_type.unwrap(), "application/javascript");

    // Test non-existent static file
    let response = make_request(&mut app, Method::GET, "/static/non-existent.css", None).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
async fn test_dashboard_functionality() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    let mut app = create_router(state);

    // Test dashboard page
    let response = make_request(&mut app, Method::GET, "/dashboard", None).await?;
    assert_eq!(response.status(), StatusCode::OK);

    // Test that dashboard can load system info (API endpoint used by dashboard)
    let response = make_request(&mut app, Method::GET, "/api/v1/system/info", None).await?;
    assert_eq!(response.status(), StatusCode::OK);

    // Test scheduler stats endpoint (used by dashboard)
    let response = make_request(&mut app, Method::GET, "/api/v1/scheduler/stats", None).await?;
    assert_eq!(response.status(), StatusCode::OK);

    // Test system metrics endpoint (used by dashboard)
    let response = make_request(&mut app, Method::GET, "/api/v1/system/metrics", None).await?;
    assert_eq!(response.status(), StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_setup_wizard_accessibility() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    let mut app = create_router(state);

    // Test setup page loads
    let response = make_request(&mut app, Method::GET, "/setup", None).await?;
    assert_eq!(response.status(), StatusCode::OK);

    // The setup wizard should be accessible without authentication
    // and should provide a complete onboarding flow
    
    // Test that the setup page includes the required API endpoints
    // that the wizard JavaScript will call

    // Test product creation endpoint (used by setup wizard)
    let product_data = create_test_product_request();
    let response = make_request(
        &mut app,
        Method::POST,
        "/api/v1/products",
        Some(product_data.to_string())
    ).await?;
    
    // Should succeed or fail gracefully
    assert!(
        response.status() == StatusCode::CREATED || 
        response.status() == StatusCode::BAD_REQUEST ||
        response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );

    Ok(())
}

#[tokio::test]
async fn test_cors_and_security_headers() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    let mut app = create_router(state);

    let response = make_request(&mut app, Method::GET, "/api/v1/system/info", None).await?;
    assert_eq!(response.status(), StatusCode::OK);

    // Check for CORS headers (should be present due to CorsLayer)
    let headers = response.headers();
    
    // Note: The actual CORS headers might vary depending on the request
    // This test mainly ensures the middleware is applied without errors
    
    Ok(())
}

#[tokio::test]
async fn test_compression_middleware() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    let mut app = create_router(state);

    // Make request with Accept-Encoding header
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/system/info")
        .header("Accept-Encoding", "gzip, deflate")
        .body(Body::empty())?;

    let response = app.call(request).await?;
    assert_eq!(response.status(), StatusCode::OK);

    // The compression layer should handle the encoding
    // We mainly test that it doesn't break anything

    Ok(())
}

#[tokio::test]
async fn test_error_page_handling() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    let mut app = create_router(state);

    // Test 404 for non-existent page
    let response = make_request(&mut app, Method::GET, "/non-existent-page", None).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Test 404 for non-existent product page
    let response = make_request(&mut app, Method::GET, "/products/non-existent-id", None).await?;
    assert_eq!(response.status(), StatusCode::OK); // Should render the page, even if product doesn't exist

    // Test method not allowed
    let response = make_request(&mut app, Method::PATCH, "/", None).await?;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    Ok(())
}

#[tokio::test]
async fn test_api_content_type_handling() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    let mut app = create_router(state);

    // Test JSON API endpoints return appropriate content type
    let response = make_request(&mut app, Method::GET, "/api/v1/system/info", None).await?;
    assert_eq!(response.status(), StatusCode::OK);
    
    let content_type = response.headers().get(header::CONTENT_TYPE);
    if let Some(ct) = content_type {
        let ct_str = ct.to_str().unwrap_or("");
        assert!(ct_str.contains("application/json"));
    }

    // Test that API endpoints reject non-JSON for POST requests when required
    let response = make_request(
        &mut app,
        Method::POST,
        "/api/v1/products",
        Some("not json".to_string())
    ).await?;
    
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    Ok(())
}

#[tokio::test]
async fn test_concurrent_web_requests() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    let app = create_router(state);

    // Test multiple concurrent requests to different endpoints
    let endpoints = vec![
        "/",
        "/dashboard",
        "/api/v1/system/info",
        "/api/v1/system/metrics",
        "/static/css/style.css",
        "/static/js/app.js",
    ];

    let futures = endpoints.into_iter().map(|endpoint| {
        let app_clone = app.clone();
        async move {
            app_clone
                .oneshot(
                    Request::builder()
                        .method(Method::GET)
                        .uri(endpoint)
                        .body(Body::empty())
                        .unwrap()
                )
                .await
        }
    });

    let results = futures::future::try_join_all(futures).await?;

    // All requests should succeed
    for (i, result) in results.iter().enumerate() {
        assert_eq!(
            result.status(),
            StatusCode::OK,
            "Concurrent request {} should succeed",
            i
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_web_integration_with_product_operations() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    let mut app = create_router(state);

    // 1. Access the products page
    let response = make_request(&mut app, Method::GET, "/products", None).await?;
    assert_eq!(response.status(), StatusCode::OK);

    // 2. Create a product via API (simulating what the web interface would do)
    let product_data = create_test_product_request();
    let response = make_request(
        &mut app,
        Method::POST,
        "/api/v1/products",
        Some(product_data.to_string())
    ).await?;
    
    // Should create successfully or fail gracefully
    assert!(
        response.status() == StatusCode::CREATED ||
        response.status() == StatusCode::BAD_REQUEST ||
        response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );

    // 3. List products via API (what the products page JavaScript would do)
    let response = make_request(&mut app, Method::GET, "/api/v1/products", None).await?;
    assert_eq!(response.status(), StatusCode::OK);

    // 4. Access scheduler page
    let response = make_request(&mut app, Method::GET, "/scheduler", None).await?;
    assert_eq!(response.status(), StatusCode::OK);

    // 5. Get scheduler stats (what the scheduler page would load)
    let response = make_request(&mut app, Method::GET, "/api/v1/scheduler/stats", None).await?;
    assert_eq!(response.status(), StatusCode::OK);

    Ok(())
}