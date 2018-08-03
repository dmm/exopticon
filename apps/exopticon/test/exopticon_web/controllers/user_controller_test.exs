defmodule ExopticonWeb.UserControllerTest do
  use ExUnit.Case
  @moduletag integration: true
  use ExopticonWeb.ConnCase

  alias Exopticon.Accounts

  @create_attrs %{
    name: "some name",
    password: "some password",
    password_hash: "some password_hash",
    username: "some username",
    timezone: "some timezone"
  }
  @update_attrs %{
    name: "some updated name",
    password: "some updated password",
    password_hash: "some updated password_hash",
    username: "some updated username",
    timezone: "some timezone"
  }
  @invalid_attrs %{name: nil, password: nil, password_hash: nil, username: nil}

  def fixture(:user) do
    {:ok, user} = Accounts.create_user(@create_attrs)
    user
  end

  setup %{conn: conn} = config do
    if username = config[:login_as] do
      user = user_fixture(username: username)
      conn = assign(conn, :current_user, user)

      {:ok, conn: conn, user: user}
    else
      :ok
    end
  end

  describe "index" do
    @describetag login_as: "some user"
    test "lists all users", %{conn: conn} do
      conn = get(conn, Routes.user_path(conn, :index))
      assert html_response(conn, 200) =~ "Listing Users"
    end
  end

  describe "new user" do
    @describetag login_as: "some user"
    test "renders form", %{conn: conn} do
      conn = get(conn, Routes.user_path(conn, :new))
      assert html_response(conn, 200) =~ "New User"
    end
  end

  describe "create user" do
    @describetag login_as: "some user"
    test "redirects to show when data is valid", %{conn: conn} do
      conn = post(conn, Routes.user_path(conn, :create), user: @create_attrs)

      assert %{id: id} = redirected_params(conn)
      assert redirected_to(conn) == Routes.user_path(conn, :show, id)

      conn = get(conn, Routes.user_path(conn, :show, id))
      assert html_response(conn, 200) =~ "Show User"
    end

    test "renders errors when data is invalid", %{conn: conn} do
      conn = post(conn, Routes.user_path(conn, :create), user: @invalid_attrs)
      assert html_response(conn, 200) =~ "New User"
    end
  end

  describe "edit user" do
    @describetag login_as: "some user"
    setup [:create_user]

    test "renders form for editing chosen user", %{conn: conn, user: user} do
      conn = get(conn, Routes.user_path(conn, :edit, user))
      assert html_response(conn, 200) =~ "Edit User"
    end
  end

  describe "update user" do
    @describetag login_as: "some user"
    setup [:create_user]

    test "redirects when data is valid", %{conn: conn, user: user} do
      conn = put(conn, Routes.user_path(conn, :update, user), user: @update_attrs)
      assert redirected_to(conn) == Routes.user_path(conn, :show, user)

      #      conn = get(conn, Routes.user_path(conn, :show, user))
      #      assert html_response(conn, 200) =~ "some updated name"
    end

    test "renders errors when data is invalid", %{conn: conn, user: user} do
      conn = put(conn, Routes.user_path(conn, :update, user), user: @invalid_attrs)
      assert html_response(conn, 200) =~ "Edit User"
    end
  end

  describe "delete user" do
    @describetag login_as: "some user"
    setup [:create_user]

    test "deletes chosen user", %{conn: conn, user: user} do
      conn = delete(conn, Routes.user_path(conn, :delete, user))
      assert redirected_to(conn) == Routes.user_path(conn, :index)

      assert_error_sent(404, fn ->
        get(conn, Routes.user_path(conn, :show, user))
      end)
    end
  end

  defp create_user(_) do
    user = fixture(:user)
    {:ok, user: user}
  end
end
