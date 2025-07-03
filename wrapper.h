#ifndef ZEP_WRAPPER_H
#define ZEP_WRAPPER_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// Create a new Zep editor instance
void* zep_create_editor(const char* root_path);

// Destroy the editor instance  
void zep_destroy_editor(void* editor_ptr);

// Initialize with text content
void zep_init_with_text(void* editor_ptr, const char* name, const char* text);

// Get text from the current buffer
size_t zep_get_text(void* editor_ptr, char* buffer, size_t buffer_size);

// Set editor to vim mode
void zep_set_vim_mode(void* editor_ptr);

// Handle key input
bool zep_handle_key(void* editor_ptr, uint32_t key, uint32_t modifiers);

// Display/render the editor
void zep_display(void* editor_ptr, float x, float y, float width, float height);

// Get current buffer text length
size_t zep_get_text_length(void* editor_ptr);

// Check if editor is in vim mode
bool zep_is_vim_mode(void* editor_ptr);

// Get cursor position
void zep_get_cursor_position(void* editor_ptr, int* line, int* column);

#ifdef __cplusplus
}
#endif

#endif // ZEP_WRAPPER_H