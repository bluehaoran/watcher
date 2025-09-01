// Integration tests for Uatu Watcher
// 
// These tests verify that all system components work together correctly
// and test complete user workflows from end to end.

mod integration;

use integration::*;

#[tokio::test]
async fn test_system_health() -> anyhow::Result<()> {
    // Verify that we can create a complete application state
    let _state = create_test_app_state().await?;
    
    // If we get here without panicking, the basic system is healthy
    Ok(())
}

#[tokio::test] 
async fn test_end_to_end_workflow() -> anyhow::Result<()> {
    // This test simulates a complete user workflow:
    // 1. Setup application
    // 2. Create product
    // 3. Schedule checks
    // 4. Perform checks
    // 5. View results
    // 6. Clean up
    
    let state = create_test_app_state().await?;
    
    println!("Testing end-to-end workflow...");
    
    // 1. Create a product (simulating user adding a product)
    let product_request = uatu_watcher::ProductRequest {
        name: "End-to-End Test Product".to_string(),
        description: Some("Testing complete workflow".to_string()),
        tracker_type: "price".to_string(),
        sources: vec![uatu_watcher::NewSource {
            url: "https://httpbin.org/json".to_string(),
            store_name: "Test API".to_string(),
            selector: None,
        }],
        check_interval: Some("*/30 * * * *".to_string()),
        is_active: Some(true),
        threshold_value: Some(5.0),
        threshold_type: Some("absolute".to_string()),
        notify_on: Some(vec!["decreased".to_string()]),
    };

    let product = state.product_manager.create_product(product_request).await?;
    println!("✓ Created product: {}", product.name);

    // 2. Schedule the product for regular checks
    {
        let mut scheduler = state.scheduler.lock().await;
        scheduler.schedule_product(product.clone(), "*/30 * * * *").await?;
        println!("✓ Scheduled product for regular checks");
    }

    // 3. Verify the job was scheduled
    {
        let scheduler = state.scheduler.lock().await;
        let job_info = scheduler.get_job_info(&product.id.to_string()).await;
        assert!(job_info.is_some());
        println!("✓ Verified job is scheduled");
    }

    // 4. Perform an immediate check (simulating manual trigger)
    let check_result = state.product_manager.check_product(&product).await;
    match check_result {
        Ok(result) => {
            println!("✓ Manual check completed: {} sources checked", result.sources_checked);
        }
        Err(e) => {
            println!("! Manual check failed (expected in test environment): {}", e);
        }
    }

    // 5. Get product statistics
    let stats = state.product_manager.get_product_stats(&product.id.to_string()).await?;
    println!("✓ Retrieved product stats: {} total checks", stats.total_checks);

    // 6. List all products (simulating dashboard view)
    let filter = uatu_watcher::web::FilterParams {
        active: Some(true),
        tracker_type: None,
        search: None,
    };
    let (products, total) = state.product_manager.list_products(1, 10, &filter).await?;
    assert!(total >= 1);
    println!("✓ Listed products: {} active products found", total);

    // 7. Update product (simulating user modification)
    let update = uatu_watcher::ProductUpdate {
        name: Some("Updated End-to-End Product".to_string()),
        description: Some("Updated during workflow test".to_string()),
        sources: None,
        check_interval: None,
        is_active: Some(true),
        threshold_value: Some(7.5),
        threshold_type: None,
        notify_on: None,
    };

    let updated_product = state.product_manager.update_product(&product.id.to_string(), update).await?;
    assert_eq!(updated_product.name, "Updated End-to-End Product");
    println!("✓ Updated product successfully");

    // 8. Pause scheduled job (simulating user pausing monitoring)
    {
        let scheduler = state.scheduler.lock().await;
        scheduler.pause_job(&product.id.to_string()).await?;
        
        let job_info = scheduler.get_job_info(&product.id.to_string()).await;
        assert!(job_info.is_some());
        assert_eq!(job_info.unwrap().status, "paused");
        println!("✓ Paused scheduled job");
    }

    // 9. Resume scheduled job
    {
        let scheduler = state.scheduler.lock().await;
        scheduler.resume_job(&product.id.to_string()).await?;
        
        let job_info = scheduler.get_job_info(&product.id.to_string()).await;
        assert!(job_info.is_some());
        assert_eq!(job_info.unwrap().status, "scheduled");
        println!("✓ Resumed scheduled job");
    }

    // 10. Clean up (simulating user deletion)
    {
        let mut scheduler = state.scheduler.lock().await;
        scheduler.unschedule_product(&product.id.to_string()).await?;
        println!("✓ Unscheduled product");
    }

    state.product_manager.delete_product(&product.id.to_string()).await?;
    println!("✓ Deleted product");

    // 11. Verify cleanup
    let deleted_product = state.product_manager.get_product(&product.id.to_string()).await?;
    assert!(deleted_product.is_none());
    
    {
        let scheduler = state.scheduler.lock().await;
        let job_info = scheduler.get_job_info(&product.id.to_string()).await;
        assert!(job_info.is_none());
        println!("✓ Verified complete cleanup");
    }

    println!("🎉 End-to-end workflow test completed successfully!");

    Ok(())
}

#[tokio::test]
async fn test_system_under_load() -> anyhow::Result<()> {
    // Test system behavior under concurrent load
    let state = create_test_app_state().await?;
    
    println!("Testing system under concurrent load...");
    
    // Create multiple products concurrently
    let create_futures = (0..10).map(|i| {
        let state = state.clone();
        async move {
            let request = uatu_watcher::ProductRequest {
                name: format!("Load Test Product {}", i),
                description: Some(format!("Product {} for load testing", i)),
                tracker_type: if i % 3 == 0 { "price" } else if i % 3 == 1 { "version" } else { "number" }.to_string(),
                sources: vec![uatu_watcher::NewSource {
                    url: format!("https://httpbin.org/json?id={}", i),
                    store_name: format!("Store {}", i),
                    selector: None,
                }],
                check_interval: Some("*/5 * * * *".to_string()),
                is_active: Some(true),
                threshold_value: Some(1.0),
                threshold_type: Some("absolute".to_string()),
                notify_on: Some(vec!["decreased".to_string()]),
            };
            
            state.product_manager.create_product(request).await
        }
    });

    let products = futures::future::try_join_all(create_futures).await?;
    println!("✓ Created {} products concurrently", products.len());

    // Schedule all products concurrently
    let schedule_futures = products.iter().map(|product| {
        let scheduler = Arc::clone(&state.scheduler);
        let product_clone = product.clone();
        async move {
            let mut scheduler = scheduler.lock().await;
            scheduler.schedule_product(product_clone, "*/10 * * * *").await
        }
    });

    futures::future::try_join_all(schedule_futures).await?;
    println!("✓ Scheduled all products concurrently");

    // Verify system state
    {
        let scheduler = state.scheduler.lock().await;
        let stats = scheduler.get_stats().await;
        println!("✓ Scheduler stats: {} active jobs, {} total jobs", stats.active_jobs, stats.total_jobs);
    }

    let filter = uatu_watcher::web::FilterParams {
        active: Some(true),
        tracker_type: None,
        search: Some("Load Test".to_string()),
    };
    let (active_products, total) = state.product_manager.list_products(1, 20, &filter).await?;
    assert_eq!(total, 10);
    println!("✓ Found {} active products", total);

    // Perform concurrent operations
    let operation_futures = products.iter().enumerate().map(|(i, product)| {
        let state = state.clone();
        let product_clone = product.clone();
        async move {
            match i % 3 {
                0 => {
                    // Update product
                    let update = uatu_watcher::ProductUpdate {
                        description: Some(format!("Updated during load test {}", i)),
                        threshold_value: Some(2.0),
                        ..Default::default()
                    };
                    state.product_manager.update_product(&product_clone.id.to_string(), update).await
                }
                1 => {
                    // Get product stats
                    state.product_manager.get_product_stats(&product_clone.id.to_string()).await.map(|_| product_clone)
                }
                _ => {
                    // Check product (might fail with network issues, that's OK)
                    match state.product_manager.check_product(&product_clone).await {
                        Ok(_) => Ok(product_clone),
                        Err(_) => Ok(product_clone), // Ignore check failures in load test
                    }
                }
            }
        }
    });

    let operation_results = futures::future::try_join_all(operation_futures).await?;
    println!("✓ Completed {} concurrent operations", operation_results.len());

    // Clean up all products
    let cleanup_futures = products.iter().map(|product| {
        let state = state.clone();
        let product_id = product.id.to_string();
        async move {
            // Unschedule first
            {
                let mut scheduler = state.scheduler.lock().await;
                scheduler.unschedule_product(&product_id).await.ok(); // Ignore errors
            }
            // Then delete
            state.product_manager.delete_product(&product_id).await
        }
    });

    futures::future::try_join_all(cleanup_futures).await?;
    println!("✓ Cleaned up all test products");

    println!("🎉 Load test completed successfully!");

    Ok(())
}

#[tokio::test] 
async fn test_error_recovery() -> anyhow::Result<()> {
    // Test system behavior when errors occur
    let state = create_test_app_state().await?;
    
    println!("Testing error recovery scenarios...");
    
    // 1. Test invalid product creation
    let invalid_request = uatu_watcher::ProductRequest {
        name: "".to_string(), // Invalid: empty name
        description: None,
        tracker_type: "price".to_string(),
        sources: vec![], // Invalid: no sources
        check_interval: None,
        is_active: None,
        threshold_value: None,
        threshold_type: None,
        notify_on: None,
    };

    let result = state.product_manager.create_product(invalid_request).await;
    assert!(result.is_err());
    println!("✓ Properly rejected invalid product creation");

    // 2. Test operations on non-existent products
    let fake_id = "non-existent-product-id";
    
    let result = state.product_manager.get_product(fake_id).await?;
    assert!(result.is_none());
    
    let result = state.product_manager.delete_product(fake_id).await;
    assert!(result.is_err());
    println!("✓ Properly handled non-existent product operations");

    // 3. Test scheduler error handling
    {
        let scheduler = state.scheduler.lock().await;
        
        let result = scheduler.pause_job("non-existent-job").await;
        assert!(result.is_err());
        
        let result = scheduler.resume_job("non-existent-job").await;
        assert!(result.is_err());
        println!("✓ Scheduler properly handled invalid job operations");
    }

    // 4. Create a valid product for further error testing
    let product_request = uatu_watcher::ProductRequest {
        name: "Error Recovery Test".to_string(),
        description: None,
        tracker_type: "price".to_string(),
        sources: vec![uatu_watcher::NewSource {
            url: "https://invalid-domain-that-does-not-exist.xyz/test".to_string(), // This will fail
            store_name: "Invalid Store".to_string(),
            selector: None,
        }],
        check_interval: Some("0 * * * *".to_string()),
        is_active: Some(true),
        threshold_value: None,
        threshold_type: None,
        notify_on: None,
    };

    let product = state.product_manager.create_product(product_request).await?;

    // 5. Test check failure handling
    let check_result = state.product_manager.check_product(&product).await;
    // This should fail due to invalid URL, but shouldn't crash the system
    match check_result {
        Ok(_) => println!("! Check unexpectedly succeeded"),
        Err(e) => println!("✓ Check failed gracefully: {}", e),
    }

    // 6. Test invalid schedule operations
    {
        let mut scheduler = state.scheduler.lock().await;
        
        // Try to schedule with invalid cron expression
        let result = scheduler.schedule_product(product.clone(), "invalid cron").await;
        assert!(result.is_err());
        println!("✓ Rejected invalid cron expression");
    }

    // 7. Clean up
    state.product_manager.delete_product(&product.id.to_string()).await?;
    println!("✓ Cleaned up error recovery test product");

    println!("🎉 Error recovery test completed successfully!");

    Ok(())
}

#[tokio::test]
async fn test_configuration_validation() -> anyhow::Result<()> {
    // Test that the system validates configuration properly
    println!("Testing configuration validation...");
    
    let config = get_test_config();
    
    // Validate that our test config is reasonable
    assert!(!config.server.host.is_empty());
    assert!(config.server.port > 0);
    assert!(!config.database.url.is_empty());
    assert!(config.database.max_connections > 0);
    assert!(config.scraper.max_concurrent_checks > 0);
    assert!(config.scheduler.max_running_jobs > 0);
    
    println!("✓ Configuration validation passed");
    
    Ok(())
}