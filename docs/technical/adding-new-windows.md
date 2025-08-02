# Adding New Windows

Step-by-step guide for adding new focusable windows to the AWS Dash application.

## Overview

Adding a new window that integrates with the focus system involves implementing the `FocusableWindow` trait and creating appropriate handlers in the main application.

## Step-by-Step Process

### 1. Determine Parameter Requirements

First, identify what parameters your window needs:

* **No parameters**: Use `SimpleShowParams`
* **Position only**: Use `PositionShowParams`  
* **Project + position**: Use `ProjectShowParams`
* **Theme**: Use `ThemeShowParams`
* **AWS Identity**: Use `IdentityShowParams`
* **Complex state**: Create custom parameter type

### 2. Implement the Window Struct

Create your window struct with necessary state:

```rust
pub struct MyNewWindow {
    pub open: bool,
    // Add other state fields as needed
}

impl Default for MyNewWindow {
    fn default() -> Self {
        Self {
            open: false,
        }
    }
}
```

### 3. Implement FocusableWindow Trait

#### For Simple Windows

```rust
impl FocusableWindow for MyNewWindow {
    type ShowParams = SimpleShowParams;
    
    fn window_id(&self) -> &'static str {
        "my_new_window"  // Unique identifier
    }
    
    fn window_title(&self) -> String {
        "My New Window".to_string()  // Title shown in window selector
    }
    
    fn is_open(&self) -> bool {
        self.open
    }
    
    fn show_with_focus(&mut self, ctx: &egui::Context, _params: Self::ShowParams, bring_to_front: bool) {
        let mut window = egui::Window::new(self.window_title())
            .open(&mut self.open);
        
        // Apply focus ordering
        window = WindowFocusManager::apply_focus_order(window, bring_to_front);
        
        window.show(ctx, |ui| {
            // Your window content here
            ui.label("Hello from my new window!");
        });
    }
}
```

#### For Parameter Windows

```rust
impl FocusableWindow for MyParameterWindow {
    type ShowParams = PositionShowParams;
    
    fn window_id(&self) -> &'static str {
        "my_parameter_window"
    }
    
    fn window_title(&self) -> String {
        "My Parameter Window".to_string()
    }
    
    fn is_open(&self) -> bool {
        self.open
    }
    
    fn show_with_focus(&mut self, ctx: &egui::Context, params: Self::ShowParams, bring_to_front: bool) {
        let mut window = egui::Window::new(self.window_title())
            .open(&mut self.open);
        
        // Use parameters
        if let Some(pos) = params.window_pos {
            window = window.current_pos(pos.min);
        }
        
        // Apply focus ordering
        window = WindowFocusManager::apply_focus_order(window, bring_to_front);
        
        window.show(ctx, |ui| {
            // Your window content here
        });
    }
}
```

### 4. Add Window to DashApp

Add your window to the main application struct:

```rust
pub struct DashApp {
    // ... existing fields
    my_new_window: MyNewWindow,
}

impl Default for DashApp {
    fn default() -> Self {
        Self {
            // ... existing fields
            my_new_window: MyNewWindow::default(),
        }
    }
}
```

### 5. Create Window Handler

Create a handler method in DashApp:

```rust
impl DashApp {
    fn handle_my_new_window(&mut self, ctx: &egui::Context) {
        handle_focusable_window(
            &mut self.my_new_window,
            ctx,
            &mut self.focus_manager,
            SimpleShowParams,
        );
    }
}
```

### 6. Add to Main Update Loop

Call your handler in the main update method:

```rust
impl eframe::App for DashApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ... existing code
        
        self.handle_my_new_window(ctx);
        
        // ... rest of update code
    }
}
```

### 7. Add to Window Selector

Add your window to the window tracking system:

```rust
impl DashApp {
    fn get_tracked_windows(&self) -> Vec<(&'static str, String, bool)> {
        vec![
            // ... existing windows
            (self.my_new_window.window_id(), self.my_new_window.window_title(), self.my_new_window.is_open()),
        ]
    }
}
```

### 8. Add Menu Item (Optional)

If you want a menu item to open your window:

```rust
impl DashApp {
    fn show_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Windows", |ui| {
                    // ... existing menu items
                    if ui.button("My New Window").clicked() {
                        self.my_new_window.open = true;
                        ui.close_menu();
                    }
                });
            });
        });
    }
}
```

## Custom Parameter Types

If your window needs custom parameters, create a new parameter type:

```rust
pub struct MyCustomShowParams {
    pub my_data: SomeDataType,
    pub window_pos: Option<egui::Rect>,
}

impl FocusableWindow for MyComplexWindow {
    type ShowParams = MyCustomShowParams;
    
    // ... implement trait methods
    
    fn show_with_focus(&mut self, ctx: &egui::Context, params: Self::ShowParams, bring_to_front: bool) {
        // Use params.my_data and params.window_pos
    }
}
```

## Testing Your Window

### 1. Add Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::dashui::window_focus::SimpleShowParams;

    #[test]
    fn test_my_new_window_trait_implementation() {
        let window = MyNewWindow::default();
        assert_eq!(window.window_id(), "my_new_window");
        assert_eq!(window.window_title(), "My New Window");
        assert!(!window.is_open());
    }
}
```

### 2. Add Integration Tests

See: [UI Component Testing Guide](ui-component-testing.md) for comprehensive testing strategies.

## Common Patterns

### Window with Validation

```rust
fn show_with_focus(&mut self, ctx: &egui::Context, params: Self::ShowParams, bring_to_front: bool) {
    // Validate before showing
    if !self.is_valid_state() {
        return;
    }
    
    let mut window = egui::Window::new(self.window_title())
        .open(&mut self.open);
    window = WindowFocusManager::apply_focus_order(window, bring_to_front);
    
    window.show(ctx, |ui| {
        // Window content
    });
}
```

### Window with Return Values

```rust
impl MyWindow {
    pub fn show_with_focus_and_result(&mut self, ctx: &egui::Context, params: Self::ShowParams, bring_to_front: bool) -> Option<MyResult> {
        let mut result = None;
        
        let mut window = egui::Window::new(self.window_title())
            .open(&mut self.open);
        window = WindowFocusManager::apply_focus_order(window, bring_to_front);
        
        window.show(ctx, |ui| {
            if ui.button("Submit").clicked() {
                result = Some(MyResult::new());
                self.open = false;
            }
        });
        
        result
    }
}
```

## Troubleshooting

### Window Not Focusing

* Check that `window_id()` returns a unique identifier
* Ensure `WindowFocusManager::apply_focus_order()` is called
* Verify handler uses `handle_focusable_window()`

### Compilation Errors

* Ensure parameter types implement required traits
* Check that all trait methods are implemented
* Verify imports for `FocusableWindow` and parameter types

### Window Not in Selector

* Check that window is added to `get_tracked_windows()`
* Verify `window_title()` returns a non-empty string
* Ensure window implements `is_open()` correctly

## Related Documentation

* [Window Focus System Overview](window-focus-system.md)
* [Trait Design Patterns](trait-patterns.md)
* [Parameter Patterns](parameter-patterns.md)
* [UI Component Testing](ui-component-testing.md)