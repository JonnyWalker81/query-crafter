#include "zep/editor.h"
#include "zep/mode_vim.h"
#include "zep/mode_standard.h"
#include "zep/imgui/display_imgui.h"
#include "zep/buffer.h"
#include "zep/window.h"
#include <cstring>
#include <memory>

using namespace Zep;

extern "C" {

struct ZepEditorWrapper {
    std::unique_ptr<ZepEditor> editor;
    ZepBuffer* current_buffer;
    
    ZepEditorWrapper(const char* root_path) 
        : editor(std::make_unique<ZepEditor>(new ZepDisplay_ImGui(), fs::path(root_path))),
          current_buffer(nullptr) 
    {
        // Register vim mode
        editor->RegisterGlobalMode(std::make_shared<ZepMode_Vim>(*editor));
        editor->RegisterGlobalMode(std::make_shared<ZepMode_Standard>(*editor));
        editor->SetGlobalMode(ZepMode_Vim::StaticName());
    }
};

// Create a new Zep editor instance
void* zep_create_editor(const char* root_path) {
    try {
        return new ZepEditorWrapper(root_path ? root_path : ".");
    } catch (...) {
        return nullptr;
    }
}

// Destroy the editor instance
void zep_destroy_editor(void* editor_ptr) {
    if (editor_ptr) {
        delete static_cast<ZepEditorWrapper*>(editor_ptr);
    }
}

// Initialize with text content
void zep_init_with_text(void* editor_ptr, const char* name, const char* text) {
    if (!editor_ptr || !name || !text) return;
    
    auto* wrapper = static_cast<ZepEditorWrapper*>(editor_ptr);
    try {
        wrapper->current_buffer = wrapper->editor->InitWithText(name, text);
    } catch (...) {
        // Handle any exceptions
    }
}

// Get text from the current buffer
size_t zep_get_text(void* editor_ptr, char* buffer, size_t buffer_size) {
    if (!editor_ptr || !buffer || buffer_size == 0) return 0;
    
    auto* wrapper = static_cast<ZepEditorWrapper*>(editor_ptr);
    if (!wrapper->current_buffer) return 0;
    
    try {
        auto text = wrapper->current_buffer->GetText();
        size_t text_size = text.size();
        size_t copy_size = std::min(text_size, buffer_size - 1);
        
        std::memcpy(buffer, text.c_str(), copy_size);
        buffer[copy_size] = '\0';
        
        return copy_size;
    } catch (...) {
        return 0;
    }
}

// Set editor to vim mode
void zep_set_vim_mode(void* editor_ptr) {
    if (!editor_ptr) return;
    
    auto* wrapper = static_cast<ZepEditorWrapper*>(editor_ptr);
    try {
        wrapper->editor->SetGlobalMode(ZepMode_Vim::StaticName());
    } catch (...) {
        // Handle exceptions
    }
}

// Handle key input
bool zep_handle_key(void* editor_ptr, uint32_t key, uint32_t modifiers) {
    if (!editor_ptr) return false;
    
    auto* wrapper = static_cast<ZepEditorWrapper*>(editor_ptr);
    if (!wrapper->current_buffer) return false;
    
    try {
        auto* mode = wrapper->current_buffer->GetMode();
        if (!mode) return false;
        
        // Convert modifiers to Zep format
        uint32_t zep_modifiers = 0;
        if (modifiers & 1) zep_modifiers |= ModifierKey::Ctrl;
        if (modifiers & 2) zep_modifiers |= ModifierKey::Alt;
        if (modifiers & 4) zep_modifiers |= ModifierKey::Shift;
        
        // Map special keys
        uint32_t zep_key = key;
        if (key == 1000) zep_key = ExtKeys::UP;
        else if (key == 1001) zep_key = ExtKeys::DOWN;
        else if (key == 1002) zep_key = ExtKeys::LEFT;
        else if (key == 1003) zep_key = ExtKeys::RIGHT;
        
        mode->AddKeyPress(zep_key, zep_modifiers);
        return true;
    } catch (...) {
        return false;
    }
}

// Display/render the editor
void zep_display(void* editor_ptr, float x, float y, float width, float height) {
    if (!editor_ptr) return;
    
    auto* wrapper = static_cast<ZepEditorWrapper*>(editor_ptr);
    try {
        // Set display region
        wrapper->editor->SetDisplayRegion(NVec2f(x, y), NVec2f(x + width, y + height));
        
        // Display the editor (this would normally render to ImGui)
        wrapper->editor->Display();
    } catch (...) {
        // Handle exceptions
    }
}

// Get current buffer text length
size_t zep_get_text_length(void* editor_ptr) {
    if (!editor_ptr) return 0;
    
    auto* wrapper = static_cast<ZepEditorWrapper*>(editor_ptr);
    if (!wrapper->current_buffer) return 0;
    
    try {
        return wrapper->current_buffer->GetText().size();
    } catch (...) {
        return 0;
    }
}

// Check if editor is in vim mode
bool zep_is_vim_mode(void* editor_ptr) {
    if (!editor_ptr) return false;
    
    auto* wrapper = static_cast<ZepEditorWrapper*>(editor_ptr);
    try {
        auto* mode = wrapper->editor->GetGlobalMode();
        return mode && (mode->Name() == ZepMode_Vim::StaticName());
    } catch (...) {
        return false;
    }
}

// Get cursor position
void zep_get_cursor_position(void* editor_ptr, int* line, int* column) {
    if (!editor_ptr || !line || !column) return;
    
    *line = 0;
    *column = 0;
    
    auto* wrapper = static_cast<ZepEditorWrapper*>(editor_ptr);
    if (!wrapper->current_buffer) return;
    
    try {
        auto* window = wrapper->editor->GetActiveWindow();
        if (window && window->GetBuffer() == wrapper->current_buffer) {
            auto cursor = window->GetBufferCursor();
            *line = (int)wrapper->current_buffer->GetLineFromOffset(cursor);
            *column = (int)(cursor - wrapper->current_buffer->GetLinePos(cursor, LineLocation::LineBegin));
        }
    } catch (...) {
        // Keep default values
    }
}

} // extern "C"