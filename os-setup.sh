sudo apt-get install \
  git \
  build-essential \
  cmake \
  qt5-default \
  clang \
  ffmpeg \
  freeglut3-dev \
  libabdevice-dev \
  libasound2 \
  libasound2-dev \
  libavcodec-dev \
  libavfilter-dev \
  libavformat-dev \
  libavutil-dev \
  libffmpeg-dev \
  libjack-jackd2-dev \
  libqt5svg5-dev \
  libqt5webkit5-dev \
  libqt5xmlpatterns5 \
  libqt5xmlpatterns5-dev \
  libsdl2-dev \
  libssl-dev \
  libxi-dev \
  libxmu-dev \
  libxrandr-dev \
  pkg-config \
  qtmultimedia5-dev \
  qtscript5-dev \
  qttools5-dev \
  qttools5-dev-tools \
  sdl \
  gcc-arm-linux-gnueabihf


### build setup
apt-get install qemu-system-arm qemu-efi

dd if=/dev/zero of=flash0.img bs=1M count=64
dd if=/usr/share/qemu-efi/QEMU_EFI.fd of=flash0.img conv=notrunc
dd if=/dev/zero of=flash1.img bs=1M count=64

sudo qemu-system-aarch64 -m 1024 -cpu cortex-a57 -M virt -nographic -pflash flash0.img -pflash flash1.img \
  -drive if=none,file=ubuntu-21.04-preinstalled-server-arm64+raspi.img.xz,id=hd0 \
  -device virtio-blk-device,drive=hd0 -netdev type=tap,id=net0 \
  -device virtio-net-device,netdev=net0,mac=E0:94:29:1F:DE:83

OPENCV_VERSION=4.5.2
BUILD_FLAGS="
    -D BUILD_CUDA_STUBS=OFF
    -D BUILD_DOCS=OFF
    -D BUILD_EXAMPLES=OFF
    -D BUILD_IPP_IW=ON
    -D BUILD_ITT=ON
    -D BUILD_JASPER=OFF
    -D BUILD_JAVA=OFF
    -D BUILD_JPEG=OFF
    -D BUILD_OPENEXR=OFF
    -D BUILD_PERF_TESTS=OFF
    -D BUILD_PNG=OFF
    -D BUILD_PROTOBUF=ON
    -D BUILD_SHARED_LIBS=ON
    -D BUILD_TBB=OFF
    -D BUILD_TESTS=OFF
    -D BUILD_TIFF=OFF
    -D BUILD_WEBP=OFF
    -D BUILD_WITH_DEBUG_INFO=OFF
    -D BUILD_WITH_DYNAMIC_IPP=OFF
    -D BUILD_ZLIB=OFF
    -D BUILD_opencv_apps=ALL
    -D BUILD_opencv_python2=OFF
    -D BUILD_opencv_python3=OFF
    -D CMAKE_BUILD_TYPE=Release
    -D CMAKE_INSTALL_PREFIX=/usr
    -D CV_DISABLE_OPTIMIZATION=OFF
    -D ENABLE_CONFIG_VERIFICATION=OFF
    -D ENABLE_FAST_MATH=OFF
    -D CV_ENABLE_INTRINSICS=ON
    -D ENABLE_LTO=OFF
    -D ENABLE_PIC=ON
    -D ENABLE_PRECOMPILED_HEADERS=OFF
    -D INSTALL_CREATE_DISTRIB=OFF
    -D INSTALL_PYTHON_EXAMPLES=OFF
    -D INSTALL_C_EXAMPLES=OFF
    -D INSTALL_TESTS=OFF
    -D OPENCV_ENABLE_NONFREE=ON
    -D OPENCV_FORCE_3RDPARTY_BUILD=OFF
    -D OPENCV_GENERATE_PKGCONFIG=OFF
    -D PROTOBUF_UPDATE_FILES=OFF
    -D WITH_1394=ON
    -D WITH_ADE=ON
    -D WITH_ARAVIS=OFF
    -D WITH_CLP=OFF
    -D WITH_CUDA=OFF
    -D WITH_EIGEN=ON
    -D WITH_FFMPEG=ON
    -D WITH_GDAL=ON
    -D WITH_GDCM=OFF
    -D WITH_GIGEAPI=OFF
    -D WITH_GPHOTO2=ON
    -D WITH_GSTREAMER=ON
    -D WITH_GSTREAMER_0_10=OFF
    -D WITH_GTK=OFF
    -D WITH_GTK_2_X=OFF
    -D WITH_HALIDE=OFF
    -D WITH_IMGCODEC_HDcR=ON
    -D WITH_IMGCODEC_PXM=ON
    -D WITH_IMGCODEC_SUNRASTER=ON
    -D WITH_INF_ENGINE=OFF
    -D WITH_IPP=ON
    -D WITH_ITT=ON
    -D WITH_JASPER=OFF
    -D WITH_JPEG=ON
    -D WITH_LAPACK=ON
    -D WITH_LIBV4L=OFF
    -D WITH_MATLAB=OFF
    -D WITH_MFX=OFF
    -D WITH_OPENCL=OFF
    -D WITH_OPENCLAMDBLAS=OFF
    -D WITH_OPENCLAMDFFT=OFF
    -D WITH_OPENCL_SVM=OFF
    -D WITH_OPENEXR=ON
    -D WITH_OPENGL=ON
    -D WITH_OPENMP=OFF
    -D WITH_OPENNI=OFF
    -D WITH_OPENNI2=OFF
    -D WITH_OPENVX=OFF
    -D WITH_PNG=ON
    -D WITH_PROTOBUF=ON
    -D WITH_PTHREADS_PF=ON
    -D WITH_PVAPI=OFF
    -D WITH_QT=ON
    -D WITH_QUIRC=ON
    -D WITH_TBB=ON
    -D WITH_TIFF=ON
    -D WITH_UNICAP=OFF
    -D WITH_V4L=ON
    -D WITH_VA=ON
    -D WITH_VA_INTEL=ON
    -D WITH_VTK=ON
    -D WITH_WEBP=ON
    -D WITH_XIMEA=OFF
    -D WITH_XINE=OFF
    -D OPENCV_ENABLE_MEMALIGN=OFF
  "
base_dir="$HOME/build/opencv/"
mkdir -p "$base_dir"

#curl -L "https://github.com/opencv/opencv/archive/$OPENCV_VERSION.tar.gz" | tar -xzC "$base_dir"
#curl -L "https://github.com/opencv/opencv_contrib/archive/$OPENCV_VERSION.tar.gz" | tar -xzC "$base_dir"

build_dir="$base_dir/opencv-$OPENCV_VERSION-build/"
mkdir -p "$build_dir"

pushd "$build_dir" > /dev/null
cmake $BUILD_FLAGS \
  -D CMAKE_INSTALL_PREFIX=/usr \
  -D OPENCV_EXTRA_MODULES_PATH="$base_dir/opencv_contrib-$OPENCV_VERSION/modules" \
  "$base_dir/opencv-$OPENCV_VERSION"
make -j"$(nproc)"
sudo make -j"$(nproc)" install
popd > /dev/null
