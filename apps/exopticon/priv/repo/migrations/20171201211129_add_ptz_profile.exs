defmodule Exopticon.Repo.Migrations.AddPtzProfile do
  use Ecto.Migration

  def change do
    alter table(:cameras) do
      add :ptz_profile_token, :string
    end
  end
end
