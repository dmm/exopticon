// Brunch automatically concatenates all files in your
// watched paths. Those paths can be configured at
// config.paths.watched in "brunch-config.js".
//
// However, those files will only be executed if
// explicitly imported. The only exception are files
// in vendor, which are never wrapped in imports and
// therefore are always executed.

// Import dependencies
//
// If you no longer want to use a dependency, remember
// to also remove its path from "config.paths.watched".
import 'phoenix_html';

// Import local files
//
// Local files can be imported directly using relative
// paths "./socket" or full ones "web/static/js/socket".
import loadView from './views/loader';

import '../css/phoenix.css';
import '../css/components/green_theme.css';
import '../css/snapshots.css';

/**
 * called on dom content loaded event, determines the current view and
 * calls the view function
 */
function handleDomContentLoaded() {
    // Get the current view name
  const viewName = document.getElementsByTagName('body')[0]
        .dataset.jsViewName;

    // Load view class and mount it
    const ViewClass = loadView(viewName);
    const view = new ViewClass();
    view.mount();

    window.currentView = view;
}

/**
 * called on document unloaded event, intiates the chain of unmount calls
 */
function handleDocumentUnload() {
    window.currentView.unmount();
}

window.addEventListener('DOMContentLoaded', handleDomContentLoaded, false);
window.addEventListener('unload', handleDocumentUnload, false);

