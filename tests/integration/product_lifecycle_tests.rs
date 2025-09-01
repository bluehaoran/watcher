use super::*;
use serde_json::json;
use uatu_watcher::{ProductRequest, NewSource};

#[tokio::test]
async fn test_complete_product_lifecycle() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    
    // 1. Create a product through ProductManager
    let product_request = ProductRequest {
        name: "Integration Test Product".to_string(),
        description: Some("Testing full lifecycle".to_string()),
        tracker_type: "price".to_string(),
        sources: vec![NewSource {
            url: "https://httpbin.org/json".to_string(),
            store_name: "Test Store".to_string(),
            selector: None,
        }],
        check_interval: Some("0 * * * *".to_string()),
        is_active: Some(true),
        threshold_value: Some(10.0),
        threshold_type: Some("absolute".to_string()),
        notify_on: Some(vec!["decreased".to_string()]),
    };

    let product = state.product_manager.create_product(product_request).await?;
    let product_id = product.id.to_string();

    // 2. Verify product was created
    let retrieved_product = state.product_manager.get_product(&product_id).await?;
    assert!(retrieved_product.is_some());
    let retrieved_product = retrieved_product.unwrap();
    assert_eq!(retrieved_product.name, "Integration Test Product");
    assert!(retrieved_product.is_active);

    // 3. Check product (this might fail with external service, but should not crash)
    let check_result = state.product_manager.check_product(&retrieved_product).await;
    
    // We don't assert success here because external HTTP calls might fail in test environment
    // But the operation should not panic or return unexpected errors
    match check_result {
        Ok(result) => {
            println!("Check succeeded: {:?}", result);
        },
        Err(e) => {
            println!("Check failed as expected in test environment: {}", e);
        }
    }

    // 4. Get product stats
    let stats = state.product_manager.get_product_stats(&product_id).await?;
    assert_eq!(stats.product_id, product_id);

    // 5. Update product
    let update = uatu_watcher::ProductUpdate {
        name: Some("Updated Integration Test Product".to_string()),
        description: Some("Updated description".to_string()),
        sources: None,
        check_interval: None,
        is_active: Some(false), // Deactivate
        threshold_value: Some(15.0),
        threshold_type: None,
        notify_on: None,
    };

    let updated_product = state.product_manager.update_product(&product_id, update).await?;
    assert_eq!(updated_product.name, "Updated Integration Test Product");
    assert!(!updated_product.is_active);

    // 6. List products (should include our product)
    let filter = uatu_watcher::web::FilterParams {
        active: None,
        tracker_type: None,
        search: None,
    };
    let (products, _total) = state.product_manager.list_products(1, 10, &filter).await?;
    assert!(!products.is_empty());
    assert!(products.iter().any(|p| p.id.to_string() == product_id));

    // 7. Delete product
    state.product_manager.delete_product(&product_id).await?;

    // 8. Verify product was deleted
    let deleted_product = state.product_manager.get_product(&product_id).await?;
    assert!(deleted_product.is_none());

    Ok(())
}

#[tokio::test]
async fn test_product_with_multiple_sources() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    
    let product_request = ProductRequest {
        name: "Multi-Source Product".to_string(),
        description: None,
        tracker_type: "price".to_string(),
        sources: vec![
            NewSource {
                url: "https://httpbin.org/json".to_string(),
                store_name: "Store 1".to_string(),
                selector: None,
            },
            NewSource {
                url: "https://httpbin.org/get".to_string(),
                store_name: "Store 2".to_string(),
                selector: Some(".price".to_string()),
            },
        ],
        check_interval: Some("0 */6 * * *".to_string()),
        is_active: Some(true),
        threshold_value: None,
        threshold_type: None,
        notify_on: None,
    };

    let product = state.product_manager.create_product(product_request).await?;
    
    // Verify both sources were added
    assert_eq!(product.sources.len(), 2);
    assert_eq!(product.sources[0].store_name, "Store 1");
    assert_eq!(product.sources[1].store_name, "Store 2");
    assert_eq!(product.sources[1].selector, Some(".price".to_string()));

    // Clean up
    state.product_manager.delete_product(&product.id.to_string()).await?;

    Ok(())
}

#[tokio::test]
async fn test_product_validation_rules() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;

    // Test with empty name (should fail)
    let invalid_request = ProductRequest {
        name: "".to_string(), // Empty name
        description: None,
        tracker_type: "price".to_string(),
        sources: vec![NewSource {
            url: "https://example.com".to_string(),
            store_name: "Test Store".to_string(),
            selector: None,
        }],
        check_interval: None,
        is_active: None,
        threshold_value: None,
        threshold_type: None,
        notify_on: None,
    };

    let result = state.product_manager.create_product(invalid_request).await;
    assert!(result.is_err());

    // Test with invalid URL (should fail)
    let invalid_url_request = ProductRequest {
        name: "Valid Name".to_string(),
        description: None,
        tracker_type: "price".to_string(),
        sources: vec![NewSource {
            url: "not-a-url".to_string(), // Invalid URL
            store_name: "Test Store".to_string(),
            selector: None,
        }],
        check_interval: None,
        is_active: None,
        threshold_value: None,
        threshold_type: None,
        notify_on: None,
    };

    let result = state.product_manager.create_product(invalid_url_request).await;
    assert!(result.is_err());

    // Test with no sources (should fail)
    let no_sources_request = ProductRequest {
        name: "Valid Name".to_string(),
        description: None,
        tracker_type: "price".to_string(),
        sources: vec![], // No sources
        check_interval: None,
        is_active: None,
        threshold_value: None,
        threshold_type: None,
        notify_on: None,
    };

    let result = state.product_manager.create_product(no_sources_request).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_product_filtering_and_search() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;

    // Create test products
    let products_data = vec![
        ("Active Price Product", "price", true),
        ("Inactive Version Product", "version", false),
        ("Active Number Product", "number", true),
    ];

    let mut created_products = Vec::new();

    for (name, tracker_type, is_active) in products_data {
        let request = ProductRequest {
            name: name.to_string(),
            description: None,
            tracker_type: tracker_type.to_string(),
            sources: vec![NewSource {
                url: "https://httpbin.org/json".to_string(),
                store_name: "Test Store".to_string(),
                selector: None,
            }],
            check_interval: None,
            is_active: Some(is_active),
            threshold_value: None,
            threshold_type: None,
            notify_on: None,
        };

        let product = state.product_manager.create_product(request).await?;
        created_products.push(product);
    }

    // Test filtering by active status
    let active_filter = uatu_watcher::web::FilterParams {
        active: Some(true),
        tracker_type: None,
        search: None,
    };
    let (active_products, _) = state.product_manager.list_products(1, 10, &active_filter).await?;
    assert_eq!(active_products.len(), 2); // Should have 2 active products

    // Test filtering by tracker type
    let price_filter = uatu_watcher::web::FilterParams {
        active: None,
        tracker_type: Some("price".to_string()),
        search: None,
    };
    let (price_products, _) = state.product_manager.list_products(1, 10, &price_filter).await?;
    assert_eq!(price_products.len(), 1); // Should have 1 price product

    // Test search functionality
    let search_filter = uatu_watcher::web::FilterParams {
        active: None,
        tracker_type: None,
        search: Some("Version".to_string()),
    };
    let (search_results, _) = state.product_manager.list_products(1, 10, &search_filter).await?;
    assert_eq!(search_results.len(), 1); // Should find the version product

    // Clean up
    for product in created_products {
        state.product_manager.delete_product(&product.id.to_string()).await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_product_concurrent_operations() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    
    // Create multiple products concurrently
    let create_futures = (0..5).map(|i| {
        let state = state.clone();
        async move {
            let request = ProductRequest {
                name: format!("Concurrent Product {}", i),
                description: None,
                tracker_type: "price".to_string(),
                sources: vec![NewSource {
                    url: "https://httpbin.org/json".to_string(),
                    store_name: "Test Store".to_string(),
                    selector: None,
                }],
                check_interval: None,
                is_active: Some(true),
                threshold_value: None,
                threshold_type: None,
                notify_on: None,
            };
            
            state.product_manager.create_product(request).await
        }
    });

    let results = futures::future::try_join_all(create_futures).await?;
    assert_eq!(results.len(), 5);

    // Verify all products were created
    let filter = uatu_watcher::web::FilterParams {
        active: None,
        tracker_type: None,
        search: Some("Concurrent".to_string()),
    };
    let (products, _) = state.product_manager.list_products(1, 20, &filter).await?;
    assert_eq!(products.len(), 5);

    // Clean up concurrently
    let delete_futures = results.into_iter().map(|product| {
        let state = state.clone();
        async move {
            state.product_manager.delete_product(&product.id.to_string()).await
        }
    });

    futures::future::try_join_all(delete_futures).await?;

    Ok(())
}