# Actions Catalog

57 action types organized by 8 categories. Use these as the `type` field in action entities.

## 1. Navigation (7)

| Action Type | Description | Typical Trigger |
|-------------|-------------|-----------------|
| `navigate` | Navigate to a route | click on link/button |
| `navigate-back` | Go to previous route | click on back button |
| `navigate-forward` | Go to next route in history | click on forward button |
| `navigate-replace` | Replace current route (no history entry) | redirect after action |
| `navigate-external` | Open external URL | click on external link |
| `navigate-hash` | Scroll to hash anchor | click on anchor link |
| `navigate-tab` | Switch between tabs/sections | click on tab |

## 2. Form (9)

| Action Type | Description | Typical Trigger |
|-------------|-------------|-----------------|
| `form-submit` | Submit form data | click submit / press Enter |
| `form-reset` | Reset form to initial state | click reset button |
| `form-validate` | Validate all fields | blur / submit |
| `field-change` | Update a single field value | input change |
| `field-focus` | Mark field as focused/touched | input focus |
| `field-blur` | Mark field as blurred, trigger validation | input blur |
| `field-clear` | Clear a single field | click clear icon |
| `field-autocomplete` | Apply autocomplete suggestion | select suggestion |
| `field-file-select` | Select file for upload | file input change |

## 3. Data (8)

| Action Type | Description | Typical Trigger |
|-------------|-------------|-----------------|
| `data-fetch` | Fetch data from API | page load / pull-to-refresh |
| `data-create` | Create a new resource | form submit |
| `data-update` | Update existing resource | form submit / inline edit |
| `data-delete` | Delete a resource | click delete + confirm |
| `data-refresh` | Re-fetch current data | click refresh / timer |
| `data-paginate` | Load next/previous page | click pagination |
| `data-sort` | Sort data by field | click column header |
| `data-filter` | Filter data by criteria | change filter input |

## 4. UI State (8)

| Action Type | Description | Typical Trigger |
|-------------|-------------|-----------------|
| `toggle-visibility` | Show/hide element | click toggle |
| `toggle-expand` | Expand/collapse section | click accordion header |
| `toggle-select` | Select/deselect item | click item / checkbox |
| `toggle-mode` | Switch UI mode (edit/view, dark/light) | click mode switch |
| `open-modal` | Open a modal/dialog | click trigger button |
| `close-modal` | Close a modal/dialog | click close / press Escape |
| `open-dropdown` | Open a dropdown menu | click dropdown trigger |
| `close-dropdown` | Close a dropdown menu | click outside / press Escape |

## 5. Search (6)

| Action Type | Description | Typical Trigger |
|-------------|-------------|-----------------|
| `search-submit` | Execute search query | submit / press Enter |
| `search-clear` | Clear search input and results | click clear |
| `search-suggest` | Fetch suggestions for current input | input change (debounced) |
| `search-select-suggestion` | Apply a search suggestion | click suggestion |
| `search-filter` | Apply filter to search results | change filter control |
| `search-paginate` | Navigate search result pages | click pagination |

## 6. Media (6)

| Action Type | Description | Typical Trigger |
|-------------|-------------|-----------------|
| `media-play` | Start media playback | click play |
| `media-pause` | Pause media playback | click pause |
| `media-seek` | Seek to position in media | drag scrubber |
| `media-volume` | Change volume level | drag volume slider |
| `media-fullscreen` | Toggle fullscreen mode | click fullscreen |
| `media-download` | Download media file | click download |

## 7. List/Collection (7)

| Action Type | Description | Typical Trigger |
|-------------|-------------|-----------------|
| `list-select` | Select item from list | click item |
| `list-multi-select` | Toggle item in multi-select | click + Ctrl/Cmd |
| `list-select-all` | Select all items | click select-all checkbox |
| `list-reorder` | Reorder items (drag & drop) | drag item |
| `list-add-item` | Add item to list | click add button |
| `list-remove-item` | Remove item from list | click remove / swipe |
| `list-edit-item` | Edit item inline | double-click / click edit |

## 8. System (6)

| Action Type | Description | Typical Trigger |
|-------------|-------------|-----------------|
| `notification-dismiss` | Dismiss a notification | click dismiss / timeout |
| `notification-action` | Act on a notification (e.g., undo) | click notification action |
| `clipboard-copy` | Copy content to clipboard | click copy button |
| `clipboard-paste` | Paste content from clipboard | Ctrl+V / click paste |
| `share` | Share content via share API | click share button |
| `print` | Print current view | click print / Ctrl+P |

## Summary

| Category | Count | Prefix |
|----------|-------|--------|
| Navigation | 7 | `navigate*` |
| Form | 9 | `form-*`, `field-*` |
| Data | 8 | `data-*` |
| UI State | 8 | `toggle-*`, `open-*`, `close-*` |
| Search | 6 | `search-*` |
| Media | 6 | `media-*` |
| List/Collection | 7 | `list-*` |
| System | 6 | Various |
| **Total** | **57** | |
