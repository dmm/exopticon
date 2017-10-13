defmodule Exopticon.Video do
  @moduledoc """
  The Video context.
  """

  import Ecto.Query, warn: false
  alias Exopticon.Repo

  alias Exopticon.Video.CameraGroup

  @doc """
  Returns the list of camera_groups.

  ## Examples

      iex> list_camera_groups()
      [%CameraGroup{}, ...]

  """
  def list_camera_groups do
    Repo.all(CameraGroup)
  end

  @doc """
  Gets a single camera_group.

  Raises `Ecto.NoResultsError` if the Camera group does not exist.

  ## Examples

      iex> get_camera_group!(123)
      %CameraGroup{}

      iex> get_camera_group!(456)
      ** (Ecto.NoResultsError)

  """
  def get_camera_group!(id), do: Repo.get!(CameraGroup, id)

  @doc """
  Creates a camera_group.

  ## Examples

      iex> create_camera_group(%{field: value})
      {:ok, %CameraGroup{}}

      iex> create_camera_group(%{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def create_camera_group(attrs \\ %{}) do
    %CameraGroup{}
    |> CameraGroup.changeset(attrs)
    |> Repo.insert()
  end

  @doc """
  Updates a camera_group.

  ## Examples

      iex> update_camera_group(camera_group, %{field: new_value})
      {:ok, %CameraGroup{}}

      iex> update_camera_group(camera_group, %{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def update_camera_group(%CameraGroup{} = camera_group, attrs) do
    camera_group
    |> CameraGroup.changeset(attrs)
    |> Repo.update()
  end

  @doc """
  Deletes a CameraGroup.

  ## Examples

      iex> delete_camera_group(camera_group)
      {:ok, %CameraGroup{}}

      iex> delete_camera_group(camera_group)
      {:error, %Ecto.Changeset{}}

  """
  def delete_camera_group(%CameraGroup{} = camera_group) do
    Repo.delete(camera_group)
  end

  @doc """
  Returns an `%Ecto.Changeset{}` for tracking camera_group changes.

  ## Examples

      iex> change_camera_group(camera_group)
      %Ecto.Changeset{source: %CameraGroup{}}

  """
  def change_camera_group(%CameraGroup{} = camera_group) do
    CameraGroup.changeset(camera_group, %{})
  end

  alias Exopticon.Video.Camera

  @doc """
  Returns the list of cameras.

  ## Examples

      iex> list_cameras()
      [%Camera{}, ...]

  """
  def list_cameras do
    Repo.all(Camera)
  end

  @doc """
  Gets a single camera.

  Raises `Ecto.NoResultsError` if the Camera does not exist.

  ## Examples

      iex> get_camera!(123)
      %Camera{}

      iex> get_camera!(456)
      ** (Ecto.NoResultsError)

  """
  def get_camera!(id), do: Repo.get!(Camera, id)

  @doc """
  Creates a camera.

  ## Examples

      iex> create_camera(%{field: value})
      {:ok, %Camera{}}

      iex> create_camera(%{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def create_camera(attrs \\ %{}) do
    %Camera{}
    |> Camera.changeset(attrs)
    |> Repo.insert()
  end

  @doc """
  Updates a camera.

  ## Examples

      iex> update_camera(camera, %{field: new_value})
      {:ok, %Camera{}}

      iex> update_camera(camera, %{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def update_camera(%Camera{} = camera, attrs) do
    camera
    |> Camera.changeset(attrs)
    |> Repo.update()
  end

  @doc """
  Deletes a Camera.

  ## Examples

      iex> delete_camera(camera)
      {:ok, %Camera{}}

      iex> delete_camera(camera)
      {:error, %Ecto.Changeset{}}

  """
  def delete_camera(%Camera{} = camera) do
    Repo.delete(camera)
  end

  @doc """
  Returns an `%Ecto.Changeset{}` for tracking camera changes.

  ## Examples

      iex> change_camera(camera)
      %Ecto.Changeset{source: %Camera{}}

  """
  def change_camera(%Camera{} = camera) do
    Camera.changeset(camera, %{})
  end

  alias Exopticon.Video.File

  @doc """
  Returns the list of files.

  ## Examples

      iex> list_files()
      [%File{}, ...]

  """
  def list_files do
    Repo.all(File)
  end

  @doc """
  Gets a single file.

  Raises `Ecto.NoResultsError` if the File does not exist.

  ## Examples

      iex> get_file!(123)
      %File{}

      iex> get_file!(456)
      ** (Ecto.NoResultsError)

  """
  def get_file!(id), do: Repo.get!(File, id)

  @doc """
  Creates a file.

  ## Examples

      iex> create_file(%{field: value})
      {:ok, %File{}}

      iex> create_file(%{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def create_file(attrs \\ %{}) do
    %File{}
    |> File.changeset(attrs)
    |> Repo.insert()
  end

  @doc """
  Updates a file.

  ## Examples

      iex> update_file(file, %{field: new_value})
      {:ok, %File{}}

      iex> update_file(file, %{field: bad_value})
      {:error, %Ecto.Changeset{}}

  """
  def update_file(%File{} = file, attrs) do
    file
    |> File.changeset(attrs)
    |> Repo.update()
  end

  @doc """
  Deletes a File.

  ## Examples

      iex> delete_file(file)
      {:ok, %File{}}

      iex> delete_file(file)
      {:error, %Ecto.Changeset{}}

  """
  def delete_file(%File{} = file) do
    Repo.delete(file)
  end

  @doc """
  Returns an `%Ecto.Changeset{}` for tracking file changes.

  ## Examples

      iex> change_file(file)
      %Ecto.Changeset{source: %File{}}

  """
  def change_file(%File{} = file) do
    File.changeset(file, %{})
  end

  @doc """
  Returns video for single camera
  """
  def get_files_between(camera_id, begin_time, end_time) do
    query =
      from(
        f in File,
        where: f.camera_id == ^camera_id and
          fragment("? && tsrange(?, ?)", f.time, ^begin_time, ^end_time) and
          not is_nil(f.end_monotonic),
        order_by: [asc: f.monotonic_index, asc: f.begin_monotonic]
      )

    Repo.all(query)
  end
end
