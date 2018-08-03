defmodule ExopticonWeb.FileControllerTest do
  use ExUnit.Case
  @moduletag integration: true
  use ExopticonWeb.ConnCase

  alias Exopticon.Video

  @create_attrs %{
    filename: "some filename",
    size: 42
  }
  @update_attrs %{
    filename: "some updated filename",
    size: 43
  }
  @invalid_attrs %{
    filename: nil,
    size: nil
  }

  def fixture(:file) do
    {:ok, test_file} = Video.create_file(@create_attrs)
    test_file
  end

  setup %{conn: conn} = config do
    if username = config[:login_as] do
      user = user_fixture(username: username)
      conn = assign(conn, :current_user, user)

      {:ok, conn: conn, user: user, login_as: username}
    else
      :ok
    end
  end


  describe "index" do
    @describetag login_as: "some user"
    test "lists all files", %{conn: conn} do
      conn = get(conn, Routes.file_path(conn, :index))
      assert html_response(conn, 200) =~ "Listing Files"
    end
  end

  describe "new file" do
    @describetag login_as: "some user"
    test "renders form", %{conn: conn} do
      conn = get(conn, Routes.file_path(conn, :new))
      assert html_response(conn, 200) =~ "New File"
    end
  end

  describe "create file" do
    @describetag login_as: "some user"
    test "redirects to show when data is valid", %{conn: conn} do
      conn = post(conn, Routes.file_path(conn, :create), file: @create_attrs)

      assert %{id: id} = redirected_params(conn)
      assert redirected_to(conn) == Routes.file_path(conn, :show, id)

      conn = get(conn, Routes.file_path(conn, :show, id))
      assert html_response(conn, 200) =~ "Show File"
    end

    test "renders errors when data is invalid", %{conn: conn} do
      conn = post(conn, Routes.file_path(conn, :create), file: @invalid_attrs)
      assert html_response(conn, 200) =~ "New File"
    end
  end

  describe "edit file" do
    @describetag login_as: "some user"
    setup [:create_file]

    test "renders form for editing chosen file", %{conn: conn, test_file: test_file} do
      conn = get(conn, Routes.file_path(conn, :edit, test_file))
      assert html_response(conn, 200) =~ "Edit File"
    end
  end

  describe "update file" do
    @describetag login_as: "some user"
    setup [:create_file]

    test "redirects when data is valid", %{conn: conn, test_file: test_file} do
      conn = put(conn, Routes.file_path(conn, :update, test_file), file: @update_attrs)
      assert redirected_to(conn) == Routes.file_path(conn, :show, test_file)

      conn = get(conn, Routes.file_path(conn, :show, test_file))
      assert html_response(conn, 200) =~ "some updated filename"
    end

    test "renders errors when data is invalid", %{conn: conn, test_file: test_file} do
      conn = put(conn, Routes.file_path(conn, :update, test_file), file: @invalid_attrs)
      assert html_response(conn, 200) =~ "Edit File"
    end
  end

  describe "delete file" do
    @describetag login_as: "some user"
    setup [:create_file]

    test "deletes chosen file", %{conn: conn, test_file: test_file} do
      conn = delete(conn, Routes.file_path(conn, :delete, test_file))
      assert redirected_to(conn) == Routes.file_path(conn, :index)

      assert_error_sent(404, fn ->
        get(conn, Routes.file_path(conn, :show, test_file))
      end)
    end
  end

  defp create_file(_) do
    file = fixture(:file)
    {:ok, test_file: file}
  end
end
