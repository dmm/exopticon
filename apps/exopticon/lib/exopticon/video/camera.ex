defmodule Exopticon.Video.Camera do
  use Ecto.Schema
  import Ecto.Changeset
  alias Exopticon.Video.Camera

  schema "cameras" do
    field(:fps, :integer)
    field(:ip, :string)
    field(:mac, :string)
    field(:name, :string)
    field(:onvif_port, :integer)
    field(:password, :string)
    field(:rtsp_url, :string)
    field(:type, :string)
    field(:username, :string)
    #    field :camera_group_id, :id
    belongs_to(:camera_group, Exopticon.Video.CameraGroup)

    timestamps()
  end

  @doc false
  def changeset(%Camera{} = camera, attrs) do
    camera
    |> cast(attrs, [:name, :ip, :onvif_port, :fps, :mac, :username, :password, :rtsp_url, :type])
    |> validate_required([:name, :ip, :fps, :mac, :username, :type])
    |> put_change(:camera_group_id, 1)
  end
end
