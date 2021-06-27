It's been about a year and a half since I wrote this, and I unfortunately didn't document this as well as I should have. But here's a quick infodump of what I remember. As for the rest... use the Source, Luke!

The image texture used by the scroll calendar display shader contains a mix of graphical elements, control metadata, and rendered text. We'll address each part in turn. You may find it helpful to look at a [sample rendered image](sample_rendered.png) as you follow along.

== Overall structure and elements of the scroll calendar ==

The scroll calendar prefab assembles the following parts together: 

* First, the background, which consists of a fixed-height header, a stretched/repeated section, and a fixed-height footer.
* Next, the date section header hackgrounds
* Finally, the calendar text. Calendar text is a monochrome image, divided into three parts, then overlapped on top of each other using the R/G/B color channels. Information encoded into the metadata section controls which color is used to display this data.

== Metadata ==

Metadata is encoded into a rectangular section in the upper-right corner of the image, encoded from right to left and top to bottom.. Each pixel encodes up to 26 bits of data; this unusual choice is due to the loss of precision incurred when vrchat interprets the pixel values as sRGB and converts to gamma colorspace (see [../src/datastream.rs](datastream.rs), in `ByteColor::from_value`). The components are documented in datastream.rs, and consist of:

* Width of metadata section (must be in the upper-rightmost corner!)
* Height of metadata section
* Overall width of the final rendered viewport, in texels
* Overall height of the final rendered viewport, in texels
* Height of the header section
* Height of the footer section
* The left and right x-coordinates of the non-scrolled border on the sides of the viewport.
* The height of the date headers
* The top and bottom y-coordinates of a section in which we blend from a non-scrolled view of the header, and a scrolled view of the header. This serves to maintain the top border of the viewport when scrolling off the header image.
* The Y-position of the point where the sides are stretched when scrolling off the header.
* X-coordinates of the color columns (3 pixels dividing into 4 columns, details described later)
* The main color palette (8 colors) - note that index zero is used for text on date headers
* Padding between elements (to avoid mipmap artifacts)
* Height of the text data section
* Y-coordinate of the top of the text data section
* Y-coordinate of the section containing the background of the text data section (the "background sample section" - this will be stretched/repeated to fill the main text area)
* Height of the background sample section
* Y-offsets of the header and footer images
* X coordinate of the day header color data (see below for details)
* X coordinate of the day header alpha data
* Y coordinate shared by the day header color and alpha data
* Width of the sliced texture used for the day header
* Width the day header should be expanded to
* The remainder consists of row data for the scrolling section (see below)

== Day header encoding ==

The header that is rendered to show the datestamp of each section is encoded as a sliced sprite. That is, we include just the left and right sides, and repeat pixels for the middle. Additionally, we split out the color and alpha data; this is mostly due to issues with how the generator script uses cairo, which uses premultiplied alpha. This caused some artifacting, so I chose to split out the alpha data. The generator script places these templates in the upper right, just to the left of the metadata section.

== Row data ==

We encode two arrays of data corresponding to horizontal rows of pixels in the text section. The first array encodes the Y-offset within the text section of the _prior_ day header. This is used to determine whether we are overlapping two day headers while scrolling. The second encodes either the palette indexes to use for the columns of text pixels, or if this column is part of a day header, includes a flag indicating this and the offset of the start of the header.