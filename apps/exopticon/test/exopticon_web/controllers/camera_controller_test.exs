defmodule ExopticonWeb.CameraControllerTest do
  use ExUnit.Case
  @moduletag integration: true
  use ExopticonWeb.ConnCase

  alias Exopticon.Video

  @create_attrs %{
    fps: 42,
    ip: "some ip",
    mac: "some mac",
    name: "some name",
    onvif_port: 42,
    password: "some password",
    rtsp_url: "some rtsp_url",
    type: "some type",
    username: "some username",
    mode: "enabled"
  }
  @update_attrs %{
    fps: 43,
    ip: "some updated ip",
    mac: "some updated mac",
    name: "some updated name",
    onvif_port: 43,
    password: "some updated password",
    rtsp_url: "some updated rtsp_url",
    type: "some updated type",
    username: "some updated username",
    mode: "disabled"
  }
  @invalid_attrs %{
    fps: nil,
    ip: nil,
    mac: nil,
    name: nil,
    onvif_port: nil,
    password: nil,
    rtsp_url: nil,
    type: nil,
    username: nil,
    mode: nil
  }

  def fixture(:camera) do
    {:ok, camera} = Video.create_camera(@create_attrs)
    camera
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

  test "requires user authentication on all actions", %{conn: conn} do
    Enum.each(
      [
        get(conn, Routes.camera_path(conn, :new))
      ],
      fn conn ->
        assert html_response(conn, 302)
        assert conn.halted
      end
    )
  end

  describe "index" do
    @describetag login_as: "some user"

    test "lists all cameras", %{conn: conn} do
      conn = get(conn, Routes.camera_path(conn, :index))
      assert html_response(conn, 200) =~ "Listing Cameras"
    end
  end

  describe "new camera" do
    @describetag login_as: "some user"

    test "renders form", %{conn: conn} do
      conn = get(conn, Routes.camera_path(conn, :new))
      assert html_response(conn, 200) =~ "New Camera"
    end
  end

  describe "create camera" do
    @describetag login_as: "some user"

    test "redirects to show when data is valid", %{conn: conn} do
      conn = post(conn, Routes.camera_path(conn, :create), camera: @create_attrs)

      assert %{id: id} = redirected_params(conn)
      assert redirected_to(conn) == Routes.camera_path(conn, :show, id)

      conn = get(conn, Routes.camera_path(conn, :show, id))
      assert html_response(conn, 200) =~ "Show Camera"
    end

    test "renders errors when data is invalid", %{conn: conn} do
      conn = post(conn, Routes.camera_path(conn, :create), camera: @invalid_attrs)
      assert html_response(conn, 200) =~ "New Camera"
    end
  end

  describe "edit camera" do
    @describetag login_as: "some user"
    setup [:create_camera]

    test "renders form for editing chosen camera", %{conn: conn, camera: camera} do
      conn = get(conn, Routes.camera_path(conn, :edit, camera))
      assert html_response(conn, 200) =~ "Edit Camera"
    end
  end

  describe "update camera" do
    @describetag login_as: "some user"
    setup [:create_camera]

    @tag login_as: "some user"
    test "redirects when data is valid", %{conn: conn, camera: camera} do
      conn = put(conn, Routes.camera_path(conn, :update, camera), camera: @update_attrs)
      assert redirected_to(conn) == Routes.camera_path(conn, :show, camera)

      conn = get(conn, Routes.camera_path(conn, :show, camera))
      assert html_response(conn, 200) =~ "some updated ip"
    end

    test "renders errors when data is invalid", %{conn: conn, camera: camera} do
      conn = put(conn, Routes.camera_path(conn, :update, camera), camera: @invalid_attrs)
      assert html_response(conn, 200) =~ "Edit Camera"
    end
  end

  @tag login_as: "some user"
  describe "delete camera" do
    setup [:create_camera]

    @tag login_as: "some user"
    test "deletes chosen camera", %{conn: conn, camera: camera} do
      conn = delete(conn, Routes.camera_path(conn, :delete, camera))
      assert redirected_to(conn) == Routes.camera_path(conn, :index)

      assert_error_sent(404, fn ->
        get(conn, Routes.camera_path(conn, :show, camera))
      end)
    end
  end

  defp create_camera(_) do
    camera = fixture(:camera)
    {:ok, camera: camera}
  end
end
