/**
* This is a template. Do not use this one in any imports. It is meant to be copied to another directory and properly implemented and changed.
*
* Note for Matvey: if you are commiting partial implementation, mark temporary not implemented functions as [[deprecated]]. 
* Also, if function can throw/raise error, add # Throws section to it's documentation.
*
* Note for Roman: Carefully read docuementation before implementing main functional.
*/

#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Margin {
    int left, top;
    int right, bottom; 
} Margin;

typedef struct RGBA {
    unsigned char r;
    unsigned char g;
    unsigned char b;
    unsigned char a; // Delete if it is not supported cuz i forgot
} RGBA;

typedef struct BmpImage {
    RGBA* ptrRawData; // array of RGBA with length = uiWidth*uiHeight
    unsigned long ulRawDataSize; // amount of bytes stored in ptrRawData

    unsigned int uiWidth; // Width in pixels
    unsigned int uiHeight; // Height in pixels
} BmpImage;

/**
*
* Reads file from fs and constructs BmpImage instance in heap.
*
* # Arguments
*   path - '\0' terminated absolute or relative path.
* # Returns 
*   pointer to instance of BmpImage
*
*
* # Note
*   Path is not not guaranteed library side check required.
*/
extern BmpImage* open_bmp(const char* path);

/**
*
* Writes BmpImage instance as bmp file according to `path`.
*
* # Arguments
*   path - '\0' terminated absolute or relative path.
*   img - pointer to instance of BmpImage.
*   
*
*
* # Note
*   Path is not not guaranteed library side check required.
*   img is guaranteed to be a correct BmpImage instance.
*/
extern void write_bmp(const char* path, BmpImage* img);

/**
*
* Converts 2D coordinate into 1D coordinate using width and height of given BmpImage.
*
* # Argmuents
*   img - pointer to the BmpImage instance
*   x, y - coordinates of the pixel
* # Returns
*   1D coordinate of corresponding pixel in ptrRawData
*
*
* # Note
*   img is guaranteed to be valid BmpImages and non nullptr.
*   x and y not guaranteed to be valid, library side check required
*/
extern unsigned long index(BmpImage* img, unsigned int x, unsigned int y);

/**
*
* Safely reads pixel from given BmpImage instance with given x and y coordinate.
*
* # Argumnets
*   img - pointer to the BmpImage instance
*   x, y - coordinates of the pixel
*
*
* # Note
*   x and y not guaranteed to be valid, library side check required.
*   img is guaranteed to be valid BmpImages and non nullptr.
*   Implementation of this function must relay on `unsigned long index(BmpImage* img, unsigned int x, unsigned int y)`
*/
extern RGBA read_pixel(BmpImage* img, unsigned int x, unsigned int y);

/**
*
* Safely sets pixel in given BmpImage instance with given x and y coordinate.
*
* # Argumnets
*   img - pointer to the BmpImage instance
*   x, y - coordinates of the pixel
*
*
* # Note
*   x and y not guaranteed to be valid, library side check required.
*   img is guaranteed to be valid BmpImages and non nullptr.
*   Implementation of this function must relay on `unsigned long index(BmpImage* img, unsigned int x, unsigned int y)`
*/
extern void set_pixel(BmpImage* img, unsigned int x, unsigned int y, RGBA color);

/**
*
* Creates BmpImage instance in heap and merges img1 and img2 pixel by pixel with blend coefficient `blend`. 
*
* # Arguments
*   img1, img2 - pointers to the BmpImage instances
*   blend - blending coefficient(decides the impact of each img1 and img2 on resulting image)
*
* # Returns
*   pointer to BmpImage, result of merge
*
*
* # Note
*   img1 and img2 are guaranteed to be valid BmpImages and non nullptr.
*   img1 and img2 must not be changed by the merge function behaviour.
*/
extern BmpImage* merge(BmpImage* img1, BmpImage* img2, float blend);

/**
*
* Creates BmpImage instance in heap and apply margin. 
* Margin consists of left, top, right, bottom.
* For example, if we have margin { 0, -50, 20, 0} that means we need a resulting image to be expanded 50 pixel up, -20 pixel from right to left.
* 
* # Arguments
*   img - pointer to the BmpImage intance
*   marg - margin structure
*   fill - color that will be placed, if new image area > old image area
*
* # Returns
*   pointer to BmpImage, result of margin
*
*
* # Note
*   img is guaranteed to be valid BmpImages and non nullptr.
*   img must not be changed by the merge function behaviour.
*/
extern BmpImage* margin(BmpImage* img, Margin marg, RGBA fill);

/**
*
* Creates BmpImage instance in heap and apply shrink. 
* Shrink - mean make it lower in width and height, but keep the shape. Like, shittify the quality.
* 
* # Arguments
*   img - pointer to the BmpImage intance
*   newWidth, newHeight - new dimensions of image
*
* # Returns
*   pointer to BmpImage, result of margin
*
*
* # Note
*   img is guaranteed to be valid BmpImages and non nullptr.
*   img must not be changed by the merge function behaviour.
*/
extern BmpImage* shrink(BmpImage* img, int newWidth, int newHeight);

#ifdef __cplusplus
}
#endif