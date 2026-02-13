//! Role filtering constants and functions for macOS Accessibility API.
//!
//! This module defines the accessibility roles used to filter elements during
//! content extraction. It distinguishes between document content roles and
//! UI chrome roles to ensure only meaningful document content is extracted.
//!
//! # Requirements
//! - Requirement 5.2: Exclude text from menu bars, toolbars, tab bars, and navigation elements
//! - Requirement 5.3: Focus extraction on elements with document-related roles
//! - Requirement 5.7: Skip elements with roles indicating UI chrome
//! - Requirement 7.2: Extract text from elements with specific roles

/// Accessibility roles that contain document content.
///
/// These roles represent UI elements that typically contain the actual
/// document content that users are working with. Elements with these
/// roles should be traversed and their text content extracted.
///
/// # Requirements
/// - Requirement 5.3: Focus extraction on elements with document-related roles
pub const DOCUMENT_ROLES: &[&str] = &[
    "AXTextArea",
    "AXTextField",
    "AXStaticText",
    "AXDocument",
    "AXWebArea",
    "AXCell",
    "AXTextMarkerRange",
    "AXScrollArea",      // Often contains document content
    "AXGroup",           // May contain text elements
    "AXLayoutArea",      // Microsoft Office document content area
];

/// Accessibility roles for UI chrome (to be excluded).
///
/// These roles represent UI elements that are part of the application's
/// chrome (menus, toolbars, buttons, etc.) rather than document content.
/// Elements with these roles should be skipped during content extraction.
///
/// Note: AXList and AXOutline are excluded here for most apps, but Slack
/// uses AXList for message content. Use `should_extract_from_role_for_app`
/// for app-specific filtering.
///
/// # Requirements
/// - Requirement 5.2: Exclude text from menu bars, toolbars, tab bars, and navigation elements
/// - Requirement 5.7: Skip elements with roles indicating UI chrome
pub const UI_CHROME_ROLES: &[&str] = &[
    "AXMenuBar",
    "AXMenu",
    "AXMenuItem",
    "AXToolbar",
    "AXTabGroup",
    "AXTab",
    "AXButton",
    "AXPopUpButton",
    "AXCheckBox",
    "AXRadioButton",
    "AXSlider",
    "AXSplitter",
    "AXStatusBar",
    "AXOutline",         // Sidebar navigation
    "AXList",            // Often navigation lists
];

/// Check if an element role should be traversed for content.
///
/// Returns `true` if the role is NOT a UI chrome role, meaning the element
/// and its children should be traversed for potential content extraction.
///
/// # Arguments
///
/// * `role` - The accessibility role string (e.g., "AXTextArea", "AXButton")
///
/// # Returns
///
/// `true` if the element should be traversed, `false` if it should be skipped.
///
/// # Examples
///
/// ```
/// use accessibility_extractor::platform::macos::roles::should_extract_from_role;
///
/// // Document roles should be traversed
/// assert!(should_extract_from_role("AXTextArea"));
/// assert!(should_extract_from_role("AXDocument"));
///
/// // UI chrome roles should be skipped
/// assert!(!should_extract_from_role("AXMenuBar"));
/// assert!(!should_extract_from_role("AXButton"));
///
/// // Unknown roles should be traversed (not in exclusion list)
/// assert!(should_extract_from_role("AXUnknownRole"));
/// ```
///
/// # Requirements
/// - Requirement 5.7: Skip elements with roles indicating UI chrome
pub fn should_extract_from_role(role: &str) -> bool {
    !UI_CHROME_ROLES.contains(&role)
}

/// Check if an element role contains extractable text.
///
/// Returns `true` if the role is one that typically contains text content
/// that should be extracted. This is a subset of roles where we actually
/// read the text value, as opposed to just traversing children.
///
/// # Arguments
///
/// * `role` - The accessibility role string (e.g., "AXTextArea", "AXButton")
///
/// # Returns
///
/// `true` if text should be extracted from this element, `false` otherwise.
///
/// # Examples
///
/// ```
/// use accessibility_extractor::platform::macos::roles::is_text_role;
///
/// // Text-containing roles
/// assert!(is_text_role("AXTextArea"));
/// assert!(is_text_role("AXTextField"));
/// assert!(is_text_role("AXStaticText"));
/// assert!(is_text_role("AXDocument"));
/// assert!(is_text_role("AXWebArea"));
/// assert!(is_text_role("AXCell"));
/// assert!(is_text_role("AXLayoutArea"));
///
/// // Non-text roles
/// assert!(!is_text_role("AXButton"));
/// assert!(!is_text_role("AXGroup"));
/// assert!(!is_text_role("AXScrollArea"));
/// ```
///
/// # Requirements
/// - Requirement 7.2: Extract text from elements with specific roles
pub fn is_text_role(role: &str) -> bool {
    matches!(role, 
        "AXTextArea" | "AXTextField" | "AXStaticText" | 
        "AXDocument" | "AXWebArea" | "AXCell" | "AXLayoutArea"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Unit Tests for DOCUMENT_ROLES constant
    // ============================================================================

    #[test]
    fn test_document_roles_contains_expected_roles() {
        // Verify all expected document roles are present
        assert!(DOCUMENT_ROLES.contains(&"AXTextArea"));
        assert!(DOCUMENT_ROLES.contains(&"AXTextField"));
        assert!(DOCUMENT_ROLES.contains(&"AXStaticText"));
        assert!(DOCUMENT_ROLES.contains(&"AXDocument"));
        assert!(DOCUMENT_ROLES.contains(&"AXWebArea"));
        assert!(DOCUMENT_ROLES.contains(&"AXCell"));
        assert!(DOCUMENT_ROLES.contains(&"AXTextMarkerRange"));
        assert!(DOCUMENT_ROLES.contains(&"AXScrollArea"));
        assert!(DOCUMENT_ROLES.contains(&"AXGroup"));
    }

    #[test]
    fn test_document_roles_count() {
        // Verify the expected number of document roles
        assert_eq!(DOCUMENT_ROLES.len(), 10);
    }

    // ============================================================================
    // Unit Tests for UI_CHROME_ROLES constant
    // ============================================================================

    #[test]
    fn test_ui_chrome_roles_contains_expected_roles() {
        // Verify all expected UI chrome roles are present
        assert!(UI_CHROME_ROLES.contains(&"AXMenuBar"));
        assert!(UI_CHROME_ROLES.contains(&"AXMenu"));
        assert!(UI_CHROME_ROLES.contains(&"AXMenuItem"));
        assert!(UI_CHROME_ROLES.contains(&"AXToolbar"));
        assert!(UI_CHROME_ROLES.contains(&"AXTabGroup"));
        assert!(UI_CHROME_ROLES.contains(&"AXTab"));
        assert!(UI_CHROME_ROLES.contains(&"AXButton"));
        assert!(UI_CHROME_ROLES.contains(&"AXPopUpButton"));
        assert!(UI_CHROME_ROLES.contains(&"AXCheckBox"));
        assert!(UI_CHROME_ROLES.contains(&"AXRadioButton"));
        assert!(UI_CHROME_ROLES.contains(&"AXSlider"));
        assert!(UI_CHROME_ROLES.contains(&"AXSplitter"));
        assert!(UI_CHROME_ROLES.contains(&"AXStatusBar"));
        assert!(UI_CHROME_ROLES.contains(&"AXOutline"));
        assert!(UI_CHROME_ROLES.contains(&"AXList"));
    }

    #[test]
    fn test_ui_chrome_roles_count() {
        // Verify the expected number of UI chrome roles
        assert_eq!(UI_CHROME_ROLES.len(), 15);
    }

    // ============================================================================
    // Unit Tests for should_extract_from_role function
    // ============================================================================

    #[test]
    fn test_should_extract_from_role_document_roles() {
        // Document roles should be extracted (not in UI chrome list)
        assert!(should_extract_from_role("AXTextArea"));
        assert!(should_extract_from_role("AXTextField"));
        assert!(should_extract_from_role("AXStaticText"));
        assert!(should_extract_from_role("AXDocument"));
        assert!(should_extract_from_role("AXWebArea"));
        assert!(should_extract_from_role("AXCell"));
        assert!(should_extract_from_role("AXTextMarkerRange"));
        assert!(should_extract_from_role("AXScrollArea"));
        assert!(should_extract_from_role("AXGroup"));
    }

    #[test]
    fn test_should_extract_from_role_ui_chrome_roles() {
        // UI chrome roles should NOT be extracted
        assert!(!should_extract_from_role("AXMenuBar"));
        assert!(!should_extract_from_role("AXMenu"));
        assert!(!should_extract_from_role("AXMenuItem"));
        assert!(!should_extract_from_role("AXToolbar"));
        assert!(!should_extract_from_role("AXTabGroup"));
        assert!(!should_extract_from_role("AXTab"));
        assert!(!should_extract_from_role("AXButton"));
        assert!(!should_extract_from_role("AXPopUpButton"));
        assert!(!should_extract_from_role("AXCheckBox"));
        assert!(!should_extract_from_role("AXRadioButton"));
        assert!(!should_extract_from_role("AXSlider"));
        assert!(!should_extract_from_role("AXSplitter"));
        assert!(!should_extract_from_role("AXStatusBar"));
        assert!(!should_extract_from_role("AXOutline"));
        assert!(!should_extract_from_role("AXList"));
    }

    #[test]
    fn test_should_extract_from_role_unknown_roles() {
        // Unknown roles should be extracted (not in exclusion list)
        assert!(should_extract_from_role("AXUnknownRole"));
        assert!(should_extract_from_role("AXCustomElement"));
        assert!(should_extract_from_role(""));
        assert!(should_extract_from_role("SomeRandomRole"));
    }

    #[test]
    fn test_should_extract_from_role_case_sensitive() {
        // Role matching should be case-sensitive
        assert!(should_extract_from_role("axmenubar")); // lowercase - not in list
        assert!(should_extract_from_role("AXMENUBAR")); // uppercase - not in list
        assert!(!should_extract_from_role("AXMenuBar")); // exact match - in list
    }

    // ============================================================================
    // Unit Tests for is_text_role function
    // ============================================================================

    #[test]
    fn test_is_text_role_text_roles() {
        // Text-containing roles should return true
        assert!(is_text_role("AXTextArea"));
        assert!(is_text_role("AXTextField"));
        assert!(is_text_role("AXStaticText"));
        assert!(is_text_role("AXDocument"));
        assert!(is_text_role("AXWebArea"));
        assert!(is_text_role("AXCell"));
        assert!(is_text_role("AXLayoutArea"));
    }

    #[test]
    fn test_is_text_role_non_text_roles() {
        // Non-text roles should return false
        assert!(!is_text_role("AXButton"));
        assert!(!is_text_role("AXGroup"));
        assert!(!is_text_role("AXScrollArea"));
        assert!(!is_text_role("AXTextMarkerRange"));
        assert!(!is_text_role("AXMenuBar"));
        assert!(!is_text_role("AXToolbar"));
    }

    #[test]
    fn test_is_text_role_unknown_roles() {
        // Unknown roles should return false
        assert!(!is_text_role("AXUnknownRole"));
        assert!(!is_text_role(""));
        assert!(!is_text_role("SomeRandomRole"));
    }

    #[test]
    fn test_is_text_role_case_sensitive() {
        // Role matching should be case-sensitive
        assert!(!is_text_role("axtextarea")); // lowercase - not a match
        assert!(!is_text_role("AXTEXTAREA")); // uppercase - not a match
        assert!(is_text_role("AXTextArea")); // exact match
    }

    // ============================================================================
    // Edge Case Tests
    // ============================================================================

    #[test]
    fn test_document_and_chrome_roles_are_disjoint() {
        // Verify that document roles and UI chrome roles don't overlap
        for doc_role in DOCUMENT_ROLES {
            assert!(
                !UI_CHROME_ROLES.contains(doc_role),
                "Role '{}' should not be in both DOCUMENT_ROLES and UI_CHROME_ROLES",
                doc_role
            );
        }
    }

    #[test]
    fn test_text_roles_are_subset_of_document_roles() {
        // All text roles should also be document roles
        let text_roles = ["AXTextArea", "AXTextField", "AXStaticText", "AXDocument", "AXWebArea", "AXCell", "AXLayoutArea"];
        for text_role in text_roles {
            assert!(
                DOCUMENT_ROLES.contains(&text_role),
                "Text role '{}' should be in DOCUMENT_ROLES",
                text_role
            );
        }
    }

    #[test]
    fn test_consistency_between_functions() {
        // If is_text_role returns true, should_extract_from_role should also return true
        let text_roles = ["AXTextArea", "AXTextField", "AXStaticText", "AXDocument", "AXWebArea", "AXCell", "AXLayoutArea"];
        for role in text_roles {
            assert!(
                is_text_role(role) && should_extract_from_role(role),
                "Role '{}' should be extractable if it's a text role",
                role
            );
        }
    }
}
