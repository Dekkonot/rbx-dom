local CollectionService = game:GetService("CollectionService")
local InsertService = game:GetService("InsertService")

--- Contains a map of instances to their assigned MeshId,
--- since we can't read it.
local meshIdMap = {}
--- We have to make a new MeshPart everytime we want to apply a new value
--- to 'MeshPart.MeshId', which could get expensive. So, we cache them here.
local meshIdCache = {}

-- Defines how to read and write properties that aren't directly scriptable.
--
-- The reflection database refers to these as having scriptability = "Custom"
return {
	Instance = {
		Attributes = {
			read = function(instance)
				return true, instance:GetAttributes()
			end,
			write = function(instance, _, value)
				local existing = instance:GetAttributes()

				for key, attr in pairs(value) do
					instance:SetAttribute(key, attr)
				end

				for key in pairs(existing) do
					if value[key] == nil then
						instance:SetAttribute(key, nil)
					end
				end

				return true
			end,
		},
		Tags = {
			read = function(instance)
				return true, CollectionService:GetTags(instance)
			end,
			write = function(instance, _, value)
				local existingTags = CollectionService:GetTags(instance)

				local unseenTags = {}
				for _, tag in ipairs(existingTags) do
					unseenTags[tag] = true
				end

				for _, tag in ipairs(value) do
					unseenTags[tag] = nil
					CollectionService:AddTag(instance, tag)
				end

				for tag in pairs(unseenTags) do
					CollectionService:RemoveTag(instance, tag)
				end

				return true
			end,
		},
	},
	LocalizationTable = {
		Contents = {
			read = function(instance, key)
				return true, instance:GetContents()
			end,
			write = function(instance, key, value)
				instance:SetContents(value)
				return true
			end,
		},
	},
	Model = {
		Scale = {
			read = function(instance, _, _)
				return true, instance:GetScale()
			end,
			write = function(instance, _, value)
				return true, instance:ScaleTo(value)
			end,
		},
	},
	MeshPart = {
		MeshId = {
			read = function(instance, _)
				return true, meshIdMap[instance]
			end,
			write = function(instance, _, value)
				local fetched, mesh = pcall(function()
					local meshPart = meshIdCache[value]
					if meshPart == nil then
						-- In a vacuum, this could cause a data race.
						-- In practice, it can only happen if creating a
						-- MeshPart takes so long a second request finishes
						-- first, which in turn assumes Roblox doesn't order
						-- these requests internally.
						meshPart = InsertService:CreateMeshPartAsync(
							value,
							Enum.CollisionFidelity.Default,
							Enum.RenderFidelity.Automatic
						)
						meshIdCache[value] = meshPart
					end
					return meshPart
				end)
				if not fetched then
					return false
				else
					meshIdMap[instance] = value
					local cFidelity, rFidelity = instance.CollisionFidelity, instance.RenderFidelity
					instance:ApplyMesh(mesh)
					instance.CollisionFidelity = cFidelity
					instance.RenderFidelity = rFidelity
					return true
				end
			end,
		},
	},
}
