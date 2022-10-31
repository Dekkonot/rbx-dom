# Roblox XML Model Format, Version 4
This is unofficial documentation for Roblox's XML model format. The XML model format is used for places (`.rbxlx` files), models (`.rbxmx` files), Roblox Studio settings, and many objects uploaded to Roblox's asset storage.

The XML model format has generally been replaced by the newer, more efficient [binary model format](/binary). Some use cases for the XML format still exist, owing to its human readability.

This documentation is incomplete. Contributions are welcome.

## Conventions

This documentation assumes basic familiarity with XML as a format.

For the purposes of this document, headers represent the name used by a tag and the name of the structure or datatype that a tag represents is noted under its heading. This is to maintain accuracy to the underlying file format.

## Underlying Structure

## roblox

## Meta

## Item

## Properties

`Properties` is a tag that exists exclusively as a child to [`Item`](#item) elements. It has no attributes.

It contains zero or more elements that represent individual properties that an Instance has. 

These elements are documented in [`Property Types`](#property-types). 

## SharedStrings

### SharedString (Storage)

*This tag is duplicated and may also act as a property. For more info, see [`SharedStrings`](#sharedstring-property) the property tag.*

## Property Types

Every tag listed under this point exists exclusively as a child to [`Properties`](#properties).

These tags act as representations of properties for Instances. The exact format of each tag is specific to the data type of the property, but there can be an arbitrary amount under each `Properties` element.

Each of these tags **requires** the attribute `name`. This **must** be the name of the property being represented by the tag. 

### string

This tag is simple, and directly contains the value of a string property. As an example, to represent the property `Name` as `Example`, this tag would look like this:

```xml
<string name="Name">Example</string>
```

### SharedString (Property)

*This tag is duplicated and may also act as a repository for a SharedString under the [`SharedStrings`](#sharedstrings) tag.*