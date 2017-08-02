defmodule Exopticon.Video.CameraGroup do
  use Ecto.Schema
  import Ecto.Changeset
  alias Exopticon.Video.CameraGroup


  schema "camera_groups" do
    field :max_storage_size, :integer
    field :name, :string
    field :storage_path, :string

    timestamps()
  end

  @doc false
  def changeset(%CameraGroup{} = camera_group, attrs) do
    camera_group
    |> cast(attrs, [:name, :storage_path, :max_storage_size])
    |> validate_required([:name, :storage_path, :max_storage_size])
  end
end
