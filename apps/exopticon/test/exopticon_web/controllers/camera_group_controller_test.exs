defmodule ExopticonWeb.CameraGroupControllerTest do
  use ExopticonWeb.ConnCase

  alias Exopticon.Video

  @create_attrs %{max_storage_size: 42, name: "some name", storage_path: "some storage_path"}
  @update_attrs %{max_storage_size: 43, name: "some updated name", storage_path: "some updated storage_path"}
  @invalid_attrs %{max_storage_size: nil, name: nil, storage_path: nil}

  def fixture(:camera_group) do
    {:ok, camera_group} = Video.create_camera_group(@create_attrs)
    camera_group
  end

  describe "index" do
    test "lists all camera_groups", %{conn: conn} do
      conn = get conn, camera_group_path(conn, :index)
      assert html_response(conn, 200) =~ "Listing Camera groups"
    end
  end

  describe "new camera_group" do
    test "renders form", %{conn: conn} do
      conn = get conn, camera_group_path(conn, :new)
      assert html_response(conn, 200) =~ "New Camera group"
    end
  end

  describe "create camera_group" do
    test "redirects to show when data is valid", %{conn: conn} do
      conn = post conn, camera_group_path(conn, :create), camera_group: @create_attrs

      assert %{id: id} = redirected_params(conn)
      assert redirected_to(conn) == camera_group_path(conn, :show, id)

      conn = get conn, camera_group_path(conn, :show, id)
      assert html_response(conn, 200) =~ "Show Camera group"
    end

    test "renders errors when data is invalid", %{conn: conn} do
      conn = post conn, camera_group_path(conn, :create), camera_group: @invalid_attrs
      assert html_response(conn, 200) =~ "New Camera group"
    end
  end

  describe "edit camera_group" do
    setup [:create_camera_group]

    test "renders form for editing chosen camera_group", %{conn: conn, camera_group: camera_group} do
      conn = get conn, camera_group_path(conn, :edit, camera_group)
      assert html_response(conn, 200) =~ "Edit Camera group"
    end
  end

  describe "update camera_group" do
    setup [:create_camera_group]

    test "redirects when data is valid", %{conn: conn, camera_group: camera_group} do
      conn = put conn, camera_group_path(conn, :update, camera_group), camera_group: @update_attrs
      assert redirected_to(conn) == camera_group_path(conn, :show, camera_group)

      conn = get conn, camera_group_path(conn, :show, camera_group)
      assert html_response(conn, 200) =~ "some updated name"
    end

    test "renders errors when data is invalid", %{conn: conn, camera_group: camera_group} do
      conn = put conn, camera_group_path(conn, :update, camera_group), camera_group: @invalid_attrs
      assert html_response(conn, 200) =~ "Edit Camera group"
    end
  end

  describe "delete camera_group" do
    setup [:create_camera_group]

    test "deletes chosen camera_group", %{conn: conn, camera_group: camera_group} do
      conn = delete conn, camera_group_path(conn, :delete, camera_group)
      assert redirected_to(conn) == camera_group_path(conn, :index)
      assert_error_sent 404, fn ->
        get conn, camera_group_path(conn, :show, camera_group)
      end
    end
  end

  defp create_camera_group(_) do
    camera_group = fixture(:camera_group)
    {:ok, camera_group: camera_group}
  end
end
