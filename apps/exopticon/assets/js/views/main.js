/**
 * View super class
 */
export default class MainView {
  /**
   * main view entry point
   */
  mount() {
    let showMenu = window.localStorage.getItem('show-menu');

    if (showMenu === 'true') {
      let menu = document.querySelector('#side-menu-container');
      menu.classList.add('shown');
    }
  }
  /**
   * main view exit callback
   */
  unmount() {
  }
}
