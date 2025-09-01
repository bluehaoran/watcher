use super::*;
use uatu_watcher::{ProductRequest, NewSource, ScheduleRequest};

#[tokio::test]
async fn test_scheduler_basic_operations() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    
    // Create a test product first
    let product_request = ProductRequest {
        name: "Scheduler Test Product".to_string(),
        description: None,
        tracker_type: "price".to_string(),
        sources: vec![NewSource {
            url: "https://httpbin.org/json".to_string(),
            store_name: "Test Store".to_string(),
            selector: None,
        }],
        check_interval: Some("0 * * * *".to_string()),
        is_active: Some(true),
        threshold_value: None,
        threshold_type: None,
        notify_on: None,
    };

    let product = state.product_manager.create_product(product_request).await?;
    let product_id = product.id.to_string();

    // Test scheduler operations
    {
        let scheduler = state.scheduler.lock().await;
        
        // Get initial stats
        let initial_stats = scheduler.get_stats().await;
        println!("Initial scheduler stats: {:?}", initial_stats);
        
        // List initial jobs
        let initial_jobs = scheduler.list_jobs().await;
        println!("Initial jobs count: {}", initial_jobs.len());
    }

    // Schedule the product
    let schedule_request = ScheduleRequest {
        product_id: product_id.clone(),
        cron_expression: "*/5 * * * *".to_string(), // Every 5 minutes
    };
    
    {
        let mut scheduler = state.scheduler.lock().await;
        scheduler.schedule_product(product.clone(), &schedule_request.cron_expression).await?;
    }

    // Verify job was scheduled
    {
        let scheduler = state.scheduler.lock().await;
        let job_info = scheduler.get_job_info(&product_id).await;
        assert!(job_info.is_some());
        
        let job_info = job_info.unwrap();
        assert_eq!(job_info.product_id, product_id);
        assert_eq!(job_info.cron_expression, "*/5 * * * *");
        assert_eq!(job_info.status, "scheduled");
    }

    // Test pause and resume
    {
        let scheduler = state.scheduler.lock().await;
        
        // Pause the job
        scheduler.pause_job(&product_id).await?;
        
        let job_info = scheduler.get_job_info(&product_id).await;
        assert!(job_info.is_some());
        assert_eq!(job_info.unwrap().status, "paused");
        
        // Resume the job
        scheduler.resume_job(&product_id).await?;
        
        let job_info = scheduler.get_job_info(&product_id).await;
        assert!(job_info.is_some());
        assert_eq!(job_info.unwrap().status, "scheduled");
    }

    // Unschedule the product
    {
        let mut scheduler = state.scheduler.lock().await;
        scheduler.unschedule_product(&product_id).await?;
    }

    // Verify job was removed
    {
        let scheduler = state.scheduler.lock().await;
        let job_info = scheduler.get_job_info(&product_id).await;
        assert!(job_info.is_none());
    }

    // Clean up
    state.product_manager.delete_product(&product_id).await?;

    Ok(())
}

#[tokio::test]
async fn test_scheduler_multiple_jobs() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    
    // Create multiple test products
    let mut products = Vec::new();
    for i in 0..3 {
        let product_request = ProductRequest {
            name: format!("Multi-Job Test Product {}", i),
            description: None,
            tracker_type: "price".to_string(),
            sources: vec![NewSource {
                url: "https://httpbin.org/json".to_string(),
                store_name: "Test Store".to_string(),
                selector: None,
            }],
            check_interval: Some("0 * * * *".to_string()),
            is_active: Some(true),
            threshold_value: None,
            threshold_type: None,
            notify_on: None,
        };

        let product = state.product_manager.create_product(product_request).await?;
        products.push(product);
    }

    // Schedule all products with different intervals
    let cron_expressions = [
        "*/5 * * * *",  // Every 5 minutes
        "*/10 * * * *", // Every 10 minutes  
        "*/15 * * * *", // Every 15 minutes
    ];

    {
        let mut scheduler = state.scheduler.lock().await;
        for (product, cron_expr) in products.iter().zip(cron_expressions.iter()) {
            scheduler.schedule_product(product.clone(), cron_expr).await?;
        }
    }

    // Verify all jobs were scheduled
    {
        let scheduler = state.scheduler.lock().await;
        let stats = scheduler.get_stats().await;
        assert!(stats.active_jobs >= 3);
        
        let jobs = scheduler.list_jobs().await;
        assert!(jobs.len() >= 3);
        
        // Verify each product has a job
        for product in &products {
            let job_info = scheduler.get_job_info(&product.id.to_string()).await;
            assert!(job_info.is_some());
        }
    }

    // Test bulk operations
    {
        let scheduler = state.scheduler.lock().await;
        
        // Pause all jobs
        for product in &products {
            scheduler.pause_job(&product.id.to_string()).await?;
        }
        
        // Verify all are paused
        for product in &products {
            let job_info = scheduler.get_job_info(&product.id.to_string()).await;
            assert!(job_info.is_some());
            assert_eq!(job_info.unwrap().status, "paused");
        }
        
        // Resume all jobs
        for product in &products {
            scheduler.resume_job(&product.id.to_string()).await?;
        }
        
        // Verify all are active again
        for product in &products {
            let job_info = scheduler.get_job_info(&product.id.to_string()).await;
            assert!(job_info.is_some());
            assert_eq!(job_info.unwrap().status, "scheduled");
        }
    }

    // Clean up
    {
        let mut scheduler = state.scheduler.lock().await;
        for product in &products {
            scheduler.unschedule_product(&product.id.to_string()).await?;
        }
    }

    for product in products {
        state.product_manager.delete_product(&product.id.to_string()).await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_scheduler_invalid_operations() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    
    let scheduler = state.scheduler.lock().await;
    
    // Test operations on non-existent product
    let fake_product_id = "non-existent-product-id";
    
    // Getting job info for non-existent product should return None
    let job_info = scheduler.get_job_info(fake_product_id).await;
    assert!(job_info.is_none());
    
    // Pausing non-existent job should return error
    let result = scheduler.pause_job(fake_product_id).await;
    assert!(result.is_err());
    
    // Resuming non-existent job should return error
    let result = scheduler.resume_job(fake_product_id).await;
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_scheduler_cron_validation() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    
    // Create a test product
    let product_request = ProductRequest {
        name: "Cron Validation Test".to_string(),
        description: None,
        tracker_type: "price".to_string(),
        sources: vec![NewSource {
            url: "https://httpbin.org/json".to_string(),
            store_name: "Test Store".to_string(),
            selector: None,
        }],
        check_interval: Some("0 * * * *".to_string()),
        is_active: Some(true),
        threshold_value: None,
        threshold_type: None,
        notify_on: None,
    };

    let product = state.product_manager.create_product(product_request).await?;

    {
        let mut scheduler = state.scheduler.lock().await;
        
        // Test valid cron expressions
        let valid_expressions = [
            "0 * * * *",       // Every hour
            "*/15 * * * *",    // Every 15 minutes
            "0 0 * * *",       // Daily at midnight
            "0 9 * * 1",       // Every Monday at 9 AM
        ];
        
        for expr in &valid_expressions {
            let result = scheduler.schedule_product(product.clone(), expr).await;
            assert!(result.is_ok(), "Valid cron expression '{}' should be accepted", expr);
            
            // Unschedule for next test
            scheduler.unschedule_product(&product.id.to_string()).await?;
        }
        
        // Test invalid cron expressions
        let invalid_expressions = [
            "invalid",         // Not a cron expression
            "* * * *",         // Too few fields
            "60 * * * *",      // Invalid minute (>59)
            "* 24 * * *",      // Invalid hour (>23)
            "* * 32 * *",      // Invalid day (>31)
        ];
        
        for expr in &invalid_expressions {
            let result = scheduler.schedule_product(product.clone(), expr).await;
            assert!(result.is_err(), "Invalid cron expression '{}' should be rejected", expr);
        }
    }

    // Clean up
    state.product_manager.delete_product(&product.id.to_string()).await?;

    Ok(())
}

#[tokio::test]
async fn test_scheduler_concurrent_operations() -> anyhow::Result<()> {
    let state = create_test_app_state().await?;
    
    // Create test products
    let mut products = Vec::new();
    for i in 0..5 {
        let product_request = ProductRequest {
            name: format!("Concurrent Scheduler Test {}", i),
            description: None,
            tracker_type: "price".to_string(),
            sources: vec![NewSource {
                url: "https://httpbin.org/json".to_string(),
                store_name: "Test Store".to_string(),
                selector: None,
            }],
            check_interval: Some("0 * * * *".to_string()),
            is_active: Some(true),
            threshold_value: None,
            threshold_type: None,
            notify_on: None,
        };

        let product = state.product_manager.create_product(product_request).await?;
        products.push(product);
    }

    // Schedule products concurrently
    let schedule_futures = products.iter().map(|product| {
        let scheduler = Arc::clone(&state.scheduler);
        let product_clone = product.clone();
        async move {
            let mut scheduler = scheduler.lock().await;
            scheduler.schedule_product(product_clone, "*/10 * * * *").await
        }
    });

    let schedule_results = futures::future::try_join_all(schedule_futures).await?;
    assert_eq!(schedule_results.len(), 5);

    // Verify all jobs are scheduled
    {
        let scheduler = state.scheduler.lock().await;
        for product in &products {
            let job_info = scheduler.get_job_info(&product.id.to_string()).await;
            assert!(job_info.is_some());
        }
    }

    // Perform concurrent pause/resume operations
    let pause_futures = products.iter().map(|product| {
        let scheduler = Arc::clone(&state.scheduler);
        let product_id = product.id.to_string();
        async move {
            let scheduler = scheduler.lock().await;
            scheduler.pause_job(&product_id).await
        }
    });

    futures::future::try_join_all(pause_futures).await?;

    // Verify all jobs are paused
    {
        let scheduler = state.scheduler.lock().await;
        for product in &products {
            let job_info = scheduler.get_job_info(&product.id.to_string()).await;
            assert!(job_info.is_some());
            assert_eq!(job_info.unwrap().status, "paused");
        }
    }

    // Clean up
    {
        let mut scheduler = state.scheduler.lock().await;
        for product in &products {
            scheduler.unschedule_product(&product.id.to_string()).await?;
        }
    }

    for product in products {
        state.product_manager.delete_product(&product.id.to_string()).await?;
    }

    Ok(())
}