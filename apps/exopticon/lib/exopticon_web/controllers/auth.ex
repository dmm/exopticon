# This file is a part of Exopticon, a free video surveillance tool. Visit
# https://exopticon.org for more information.
#
# Copyright (C) 2018 David Matthew Mattli
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

defmodule ExopticonWeb.Auth do
  @moduledoc """
  Provides authentication related functionality
  """
  import Plug.Conn
  import Comeonin.Bcrypt, only: [checkpw: 2, dummy_checkpw: 0]
  import Phoenix.Controller
  alias Exopticon.Accounts.User
  alias ExopticonWeb.Router.Helpers
  alias Phoenix.Token

  def init(opts) do
    Keyword.fetch(opts, :repo)
  end

  def call(conn, {:ok, repo}) do
    user_id = get_session(conn, :user_id)
    env = Application.get_env(:exopticon, :env)

    cond do
      user = env == :test && conn.assigns[:current_user] ->
        login(conn, user)

      user = user_id && repo.get(Exopticon.Accounts.User, user_id) ->
        put_current_user(conn, user)

      true ->
        assign(conn, :current_user, nil)
    end
  end

  def login(conn, user) do
    conn
    |> put_current_user(user)
    |> put_session(:user_id, user.id)
    |> configure_session(renew: true)
  end

  defp put_current_user(conn, user) do
    token = Token.sign(conn, "user socket", user.id)

    conn
    |> assign(:current_user, user)
    |> assign(:timezone, user.timezone)
    |> assign(:user_token, token)
  end

  def login_by_username_and_pass(conn, username, given_pass, opts) do
    repo = Keyword.fetch!(opts, :repo)
    user = repo.get_by(User, username: username)

    cond do
      user && checkpw(given_pass, user.password_hash) ->
        {:ok, login(conn, user)}

      user ->
        {:error, :unauthorized, conn}

      true ->
        dummy_checkpw()
        {:error, :not_found, conn}
    end
  end

  def logout(conn) do
    configure_session(conn, drop: true)
  end

  def authenticate_user(conn, _opts) do
    if conn.assigns.current_user do
      conn
    else
      conn
      |> put_flash(:error, "You must be logged in to access that page")
      |> redirect(to: Helpers.session_path(conn, :new))
      |> halt()
    end
  end
end
