
import MainView from './main';
import PageIndexView from './page_index_view.js';
import PageDevice_settingsView from './device_settings.js';
import FileBrowseView from './file/browse';
import FileShowView from './file/show';
import CameraShowView from './camera/show';

// Collection of specific view modules
const views = {
  PageIndexView,
  PageDevice_settingsView,
  FileShowView,
  CameraShowView,
  FileBrowseView,
};
/**
 * returns the implementation behind the supplied view name
 * @param {string} viewName
 * @return {function}
 */
export default function loadView(viewName) {
  return views[viewName] || MainView;
}
