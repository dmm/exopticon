defmodule Exopticon.Repo.Migrations.CreateCameras do
  use Ecto.Migration

  def change do
    create table(:cameras) do
      add(:name, :string)
      add(:ip, :string)
      add(:onvif_port, :integer)
      add(:fps, :integer)
      add(:mac, :string)
      add(:username, :string)
      add(:password, :string)
      add(:rtsp_url, :string)
      add(:type, :string)
      add(:camera_group_id, references(:camera_groups, on_delete: :nothing))

      timestamps()
    end

    create(index(:cameras, [:camera_group_id]))
  end
end
