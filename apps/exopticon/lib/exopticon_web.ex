defmodule ExopticonWeb do
  @moduledoc """
  The entrypoint for defining your web interface, such
  as controllers, views, channels and so on.

  This can be used in your application as:

      use ExopticonWeb, :controller
      use ExopticonWeb, :view

  The definitions below will be executed for every view,
  controller, etc, so keep them short and clean, focused
  on imports, uses and aliases.

  Do NOT define functions inside the quoted expressions
  below. Instead, define any helper function in modules
  and import those modules here.
  """

  def controller do
    quote do
      use Phoenix.Controller, namespace: ExopticonWeb
      import Plug.Conn
      import ExopticonWeb.Router.Helpers
      import ExopticonWeb.Gettext
      import ExopticonWeb.Auth, only: [authenticate_user: 2]
    end
  end

  def view do
    quote do
      use Phoenix.View,
        root: "lib/exopticon_web/templates",
        namespace: ExopticonWeb

      # Import convenience functions from controllers
      import Phoenix.Controller, only: [get_flash: 2, view_module: 1]

      # Use all HTML functionality (forms, tags, etc)
      use Phoenix.HTML

      import ExopticonWeb.Router.Helpers
      import ExopticonWeb.ErrorHelpers
      import ExopticonWeb.Gettext
    end
  end

  def router do
    quote do
      use Phoenix.Router
      import Plug.Conn
      import Phoenix.Controller
      import ExopticonWeb.Auth, only: [authenticate_user: 2]
    end
  end

  def channel do
    quote do
      use Phoenix.Channel
      import ExopticonWeb.Gettext
    end
  end

  @doc """
  When used, dispatch to the appropriate controller/view/etc.
  """
  defmacro __using__(which) when is_atom(which) do
    apply(__MODULE__, which, [])
  end
end
