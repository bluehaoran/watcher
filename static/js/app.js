// Global application utilities and shared functionality

// API utility functions
class ApiClient {
    static async get(url) {
        try {
            const response = await fetch(url);
            return await response.json();
        } catch (error) {
            console.error('API GET error:', error);
            throw error;
        }
    }

    static async post(url, data = null) {
        try {
            const options = {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
            };
            
            if (data) {
                options.body = JSON.stringify(data);
            }

            const response = await fetch(url, options);
            return await response.json();
        } catch (error) {
            console.error('API POST error:', error);
            throw error;
        }
    }

    static async put(url, data) {
        try {
            const response = await fetch(url, {
                method: 'PUT',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(data)
            });
            return await response.json();
        } catch (error) {
            console.error('API PUT error:', error);
            throw error;
        }
    }

    static async delete(url) {
        try {
            const response = await fetch(url, {
                method: 'DELETE'
            });
            return await response.json();
        } catch (error) {
            console.error('API DELETE error:', error);
            throw error;
        }
    }
}

// Flash message utility
class FlashMessages {
    static show(message, type = 'info', duration = 5000) {
        const flashContainer = this.getOrCreateContainer();
        
        const alert = document.createElement('div');
        alert.className = `alert alert-${type}`;
        alert.innerHTML = `
            ${message}
            <button class="alert-close" onclick="this.parentElement.remove()">&times;</button>
        `;
        
        flashContainer.appendChild(alert);
        
        // Auto-remove after duration
        if (duration > 0) {
            setTimeout(() => {
                if (alert.parentElement) {
                    alert.remove();
                }
            }, duration);
        }

        return alert;
    }

    static success(message, duration = 5000) {
        return this.show(message, 'success', duration);
    }

    static error(message, duration = 8000) {
        return this.show(message, 'error', duration);
    }

    static warning(message, duration = 6000) {
        return this.show(message, 'warning', duration);
    }

    static info(message, duration = 5000) {
        return this.show(message, 'info', duration);
    }

    static getOrCreateContainer() {
        let container = document.querySelector('.flash-messages');
        
        if (!container) {
            container = document.createElement('div');
            container.className = 'flash-messages';
            
            // Insert after navbar or at top of container
            const insertPoint = document.querySelector('.page-header') || 
                               document.querySelector('.container') || 
                               document.body.firstChild;
            
            if (insertPoint && insertPoint.parentNode) {
                insertPoint.parentNode.insertBefore(container, insertPoint);
            } else {
                document.body.prepend(container);
            }
        }
        
        return container;
    }

    static clear() {
        const container = document.querySelector('.flash-messages');
        if (container) {
            container.innerHTML = '';
        }
    }
}

// Form validation utilities
class FormValidator {
    static validateRequired(value, fieldName) {
        if (!value || value.trim() === '') {
            throw new Error(`${fieldName} is required`);
        }
        return true;
    }

    static validateUrl(value, fieldName = 'URL') {
        if (!value) return true; // Optional field
        
        try {
            new URL(value);
            return true;
        } catch {
            throw new Error(`${fieldName} must be a valid URL`);
        }
    }

    static validateEmail(value, fieldName = 'Email') {
        if (!value) return true; // Optional field
        
        const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
        if (!emailRegex.test(value)) {
            throw new Error(`${fieldName} must be a valid email address`);
        }
        return true;
    }

    static validateCron(value, fieldName = 'Schedule') {
        if (!value) return true; // Optional field
        
        // Basic cron expression validation (simplified)
        const cronRegex = /^(\*|[0-5]?\d)(\s+(\*|[0-5]?\d)){4}$/;
        if (!cronRegex.test(value.trim())) {
            throw new Error(`${fieldName} must be a valid cron expression (e.g., "0 * * * *")`);
        }
        return true;
    }

    static validateNumber(value, fieldName = 'Number', min = null, max = null) {
        if (!value) return true; // Optional field
        
        const num = parseFloat(value);
        if (isNaN(num)) {
            throw new Error(`${fieldName} must be a valid number`);
        }
        
        if (min !== null && num < min) {
            throw new Error(`${fieldName} must be at least ${min}`);
        }
        
        if (max !== null && num > max) {
            throw new Error(`${fieldName} must be no more than ${max}`);
        }
        
        return true;
    }
}

// Date/time formatting utilities
class DateFormatter {
    static formatDateTime(dateString) {
        if (!dateString) return 'N/A';
        
        const date = new Date(dateString);
        if (isNaN(date.getTime())) return 'Invalid Date';
        
        return date.toLocaleDateString() + ' ' + date.toLocaleTimeString();
    }

    static formatDate(dateString) {
        if (!dateString) return 'N/A';
        
        const date = new Date(dateString);
        if (isNaN(date.getTime())) return 'Invalid Date';
        
        return date.toLocaleDateString();
    }

    static formatTime(dateString) {
        if (!dateString) return 'N/A';
        
        const date = new Date(dateString);
        if (isNaN(date.getTime())) return 'Invalid Date';
        
        return date.toLocaleTimeString();
    }

    static formatRelativeTime(dateString) {
        if (!dateString) return 'N/A';
        
        const date = new Date(dateString);
        if (isNaN(date.getTime())) return 'Invalid Date';
        
        const now = new Date();
        const diffMs = now - date;
        const diffSec = Math.floor(diffMs / 1000);
        const diffMin = Math.floor(diffSec / 60);
        const diffHour = Math.floor(diffMin / 60);
        const diffDay = Math.floor(diffHour / 24);

        if (diffSec < 60) {
            return 'Just now';
        } else if (diffMin < 60) {
            return `${diffMin} minute${diffMin !== 1 ? 's' : ''} ago`;
        } else if (diffHour < 24) {
            return `${diffHour} hour${diffHour !== 1 ? 's' : ''} ago`;
        } else if (diffDay < 7) {
            return `${diffDay} day${diffDay !== 1 ? 's' : ''} ago`;
        } else {
            return this.formatDate(dateString);
        }
    }

    static formatDuration(ms) {
        if (!ms || ms < 0) return '0ms';
        
        if (ms < 1000) {
            return `${Math.round(ms)}ms`;
        } else if (ms < 60000) {
            return `${(ms / 1000).toFixed(1)}s`;
        } else if (ms < 3600000) {
            return `${(ms / 60000).toFixed(1)}m`;
        } else {
            return `${(ms / 3600000).toFixed(1)}h`;
        }
    }
}

// Local storage utilities
class Storage {
    static get(key, defaultValue = null) {
        try {
            const value = localStorage.getItem(`uatu_watcher_${key}`);
            return value ? JSON.parse(value) : defaultValue;
        } catch {
            return defaultValue;
        }
    }

    static set(key, value) {
        try {
            localStorage.setItem(`uatu_watcher_${key}`, JSON.stringify(value));
            return true;
        } catch {
            return false;
        }
    }

    static remove(key) {
        try {
            localStorage.removeItem(`uatu_watcher_${key}`);
            return true;
        } catch {
            return false;
        }
    }

    static clear() {
        try {
            const keys = Object.keys(localStorage).filter(key => 
                key.startsWith('uatu_watcher_')
            );
            keys.forEach(key => localStorage.removeItem(key));
            return true;
        } catch {
            return false;
        }
    }
}

// Debounce utility for search/filter functions
function debounce(func, delay) {
    let timeoutId;
    return function (...args) {
        clearTimeout(timeoutId);
        timeoutId = setTimeout(() => func.apply(this, args), delay);
    };
}

// Throttle utility for scroll/resize handlers
function throttle(func, delay) {
    let timeoutId;
    let lastExecTime = 0;
    return function (...args) {
        const currentTime = Date.now();
        
        if (currentTime - lastExecTime > delay) {
            func.apply(this, args);
            lastExecTime = currentTime;
        } else {
            clearTimeout(timeoutId);
            timeoutId = setTimeout(() => {
                func.apply(this, args);
                lastExecTime = Date.now();
            }, delay - (currentTime - lastExecTime));
        }
    };
}

// Copy to clipboard utility
async function copyToClipboard(text) {
    try {
        if (navigator.clipboard && window.isSecureContext) {
            await navigator.clipboard.writeText(text);
            return true;
        } else {
            // Fallback for older browsers
            const textArea = document.createElement('textarea');
            textArea.value = text;
            textArea.style.position = 'fixed';
            textArea.style.opacity = '0';
            document.body.appendChild(textArea);
            textArea.focus();
            textArea.select();
            
            try {
                const successful = document.execCommand('copy');
                document.body.removeChild(textArea);
                return successful;
            } catch {
                document.body.removeChild(textArea);
                return false;
            }
        }
    } catch {
        return false;
    }
}

// HTML escaping utility
function escapeHtml(text) {
    if (!text) return '';
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Loading state manager
class LoadingManager {
    static show(element, message = 'Loading...') {
        if (typeof element === 'string') {
            element = document.querySelector(element);
        }
        
        if (element) {
            element.innerHTML = `<div class="loading-spinner">${escapeHtml(message)}</div>`;
        }
    }

    static hide(element) {
        if (typeof element === 'string') {
            element = document.querySelector(element);
        }
        
        if (element) {
            const spinner = element.querySelector('.loading-spinner');
            if (spinner) {
                spinner.remove();
            }
        }
    }

    static wrap(element, promise, message = 'Loading...') {
        this.show(element, message);
        
        return promise
            .finally(() => {
                this.hide(element);
            });
    }
}

// Confirmation dialog utility
function confirmAction(message, title = 'Confirm Action') {
    return new Promise((resolve) => {
        const result = confirm(`${title}\n\n${message}`);
        resolve(result);
    });
}

// Initialize global event handlers
document.addEventListener('DOMContentLoaded', function() {
    // Close modals when clicking outside
    document.addEventListener('click', function(event) {
        const modal = event.target.closest('.modal');
        if (modal && event.target === modal) {
            const closeBtn = modal.querySelector('.modal-close');
            if (closeBtn) {
                closeBtn.click();
            }
        }
    });

    // Keyboard shortcuts
    document.addEventListener('keydown', function(event) {
        // Close modal with Escape key
        if (event.key === 'Escape') {
            const visibleModal = document.querySelector('.modal[style*="block"]');
            if (visibleModal) {
                const closeBtn = visibleModal.querySelector('.modal-close');
                if (closeBtn) {
                    closeBtn.click();
                }
            }
        }
    });

    // Auto-hide flash messages after interaction
    document.addEventListener('click', function(event) {
        if (event.target.matches('.alert-close')) {
            const alert = event.target.closest('.alert');
            if (alert) {
                alert.remove();
            }
        }
    });

    // Form validation on submit
    document.addEventListener('submit', function(event) {
        const form = event.target;
        if (form.hasAttribute('data-validate')) {
            // Custom form validation logic can be added here
        }
    });

    // Auto-save form data to local storage
    const autoSaveForms = document.querySelectorAll('[data-autosave]');
    autoSaveForms.forEach(form => {
        const formId = form.getAttribute('data-autosave');
        
        // Load saved data
        const savedData = Storage.get(`form_${formId}`, {});
        Object.keys(savedData).forEach(name => {
            const field = form.querySelector(`[name="${name}"]`);
            if (field && field.value === '') {
                field.value = savedData[name];
            }
        });

        // Save data on change
        const debouncedSave = debounce(() => {
            const formData = new FormData(form);
            const data = {};
            for (let [name, value] of formData.entries()) {
                data[name] = value;
            }
            Storage.set(`form_${formId}`, data);
        }, 1000);

        form.addEventListener('input', debouncedSave);
        form.addEventListener('change', debouncedSave);

        // Clear saved data on successful submit
        form.addEventListener('submit', () => {
            setTimeout(() => {
                Storage.remove(`form_${formId}`);
            }, 2000);
        });
    });
});

// Export utilities for use in other scripts
window.UatuWatcher = {
    ApiClient,
    FlashMessages,
    FormValidator,
    DateFormatter,
    Storage,
    LoadingManager,
    debounce,
    throttle,
    copyToClipboard,
    escapeHtml,
    confirmAction
};